use std::{fs::File, io::Read, path::PathBuf};

use serde_json::json;
use tar::Archive;
use tempfile::TempDir;
use warpforge_api::formula::FormulaAndContext;

use crate::{
	tests::{default_context, run_formula_collect_output},
	Digest, Output,
};

#[test]
fn formula_exec_runc_output() {
	let temp_dir = TempDir::new().unwrap();

	let formula_and_context: FormulaAndContext = serde_json::from_value(json!({
		"formula": {
			"formula.v1": {
				"image": {
					"reference": "docker.io/busybox:latest",
					"readonly": true,
				},
				"inputs": {},
				"action": {
					"script": {
						"interpreter": "/bin/sh",
						"contents": [
							"echo \"hello, warpforge!\" > /out/test.txt",
						]
					}
				},
				"outputs": {
					"output.tar": {
						"from": "/out",
						"packtype": "tar"
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
		name: "output.tar".into(),
		digest: Digest::Sha384("39906bae799176280345a22a29458b2567f6a1ca373c5483d4cbae0fb0c224c519727f83f7beadf9f7b85731668ad2a1".into())
	}]);

	// Unpack output.tar and check contents.
	let reader = File::open(temp_dir.path().join("output.tar")).unwrap();
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
fn formula_exec_runc_multiple_outputs() {
	let temp_dir = TempDir::new().unwrap();

	let formula_and_context: FormulaAndContext = serde_json::from_value(json!({
		"formula": {
			"formula.v1": {
				"image": {
					"reference": "docker.io/busybox:latest",
					"readonly": true,
				},
				"inputs": {},
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
					"output_1.tar": {
						"from": "/out/1",
						"packtype": "tar"
					},
					"output_2.tar": {
						"from": "/out/2",
						"packtype": "tar"
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
			name: "output_1.tar".into(),
			digest: Digest::Sha384("94fce6489a4060aefb303bfc8e2d4b89e02860f904c38a717da8189c68d88d9a1bd32641f71ffa648f496c6cc837507f".into())
		},
		Output {
			name: "output_2.tar".into(),
			digest: Digest::Sha384("120620d4ad89b8f9d7c1e48ca4a67e6f884d7a3ce9940f3a6747d2afab3b987775bbd3f2099711cf57139b429c7d1618".into())
		},
	]);
}
