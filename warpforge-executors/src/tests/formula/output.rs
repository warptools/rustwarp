use std::{fs::File, io::Read, path::PathBuf};

use flate2::read::GzDecoder;
use serde_json::json;
use tar::Archive;
use tempfile::TempDir;
use warpforge_api::formula::FormulaAndContext;

use crate::{
	tests::{default_context, run_formula_collect_output},
	Digest, Output,
};

#[test]
fn tgz_output() {
	let temp_dir = TempDir::new().unwrap();

	let formula_and_context: FormulaAndContext = serde_json::from_value(json!({
		"formula": {
			"formula.v1": {
				"inputs": {
					"/": "oci:docker.io/library/busybox@sha256:22f27168517de1f58dae0ad51eacf1527e7e7ccc47512d3946f56bdbe913f564",
				},
				"action": {
					"script": {
						"interpreter": "/bin/sh",
						"contents": [
							"echo \"hello, warpforge!\" > /out/test.txt",
						]
					}
				},
				"outputs": {
					"output.tgz": {
						"from": "/out",
						"packtype": "tgz"
					},
				},
			}
		},
		"context": {
			"context.v1": {
				"warehouses": {}
			}
		}
	}))
	.expect("failed to parse formula json");

	let mut context = default_context();
	context.output_path = Some(temp_dir.path().into());

	let result = run_formula_collect_output(formula_and_context, &context).unwrap();

	assert_eq!(result.exit_code, Some(0));
	assert_eq!(result.outputs, vec![Output {
		name: "output.tgz".into(),
		digest: Digest::Sha384("64518bf7b504749270619507457adc3c86d46ccbb86c8b06508591aed483c1a5db728086dba261ff05f453dfd2c315d5".into())
	}]);

	// Unpack output.tar and check contents.
	let reader = File::open(temp_dir.path().join("output.tgz")).unwrap();
	let reader = GzDecoder::new(reader);
	let mut archive = Archive::new(reader);
	let mut entries = archive.entries().unwrap();
	let mut entry = entries.next().unwrap().unwrap();

	assert_eq!(entry.path().unwrap(), PathBuf::from("test.txt"));
	let mut content = String::new();
	entry.read_to_string(&mut content).unwrap();
	assert_eq!(content, "hello, warpforge!\n");
	assert!(entries.next().is_none());
}

#[test]
fn multiple_tgz_outputs() {
	let temp_dir = TempDir::new().unwrap();

	let formula_and_context: FormulaAndContext = serde_json::from_value(json!({
		"formula": {
			"formula.v1": {
				"inputs": {
					"/": "oci:docker.io/library/busybox@sha256:22f27168517de1f58dae0ad51eacf1527e7e7ccc47512d3946f56bdbe913f564",
				},
				"action": {
					"script": {
						"interpreter": "/bin/sh",
						"contents": [
							"echo \"hello, warpforge! (1)\" > /out/1/test.txt",
							"echo \"hello, warpforge!(2)\" > /out/2/test.txt",
							"mkdir /out/2/subdir",
							"echo \"hello, subdir!\" > /out/2/subdir/file.txt",
						]
					}
				},
				"outputs": {
					"output_1.tgz": {
						"from": "/out/1",
						"packtype": "tgz"
					},
					"output_2.tgz": {
						"from": "/out/2",
						"packtype": "tgz"
					},
				},
			}
		},
		"context": {
			"context.v1": {
				"warehouses": {}
			}
		}
	}))
	.expect("failed to parse formula json");

	let mut context = default_context();
	context.output_path = Some(temp_dir.path().into());

	let result = run_formula_collect_output(formula_and_context, &context).unwrap();

	assert_eq!(result.exit_code, Some(0));
	assert_eq!(result.outputs, vec![
		Output {
			name: "output_1.tgz".into(),
			digest: Digest::Sha384("dbdb8a42228f80b47f18dceab1994e59820ef26fd5b74db9e7298b77907ba25c00f6920d893670d0bc366c2ed4052047".into())
		},
		Output {
			name: "output_2.tgz".into(),
			digest: Digest::Sha384("885741449883286ea479ac9e71a7cab2b8f75cf25960f88168c56f3645f39c206f59932e20735f91e39ccb892f62b529".into())
		},
	]);
}
