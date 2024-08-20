use std::{fs::File, io::Read, path::PathBuf};

use serde_json::json;
use tar::Archive;
use tempfile::TempDir;
use warpforge_api::formula::FormulaAndContext;

use crate::tests::{default_context, run_formula_collect_output};

#[tokio::test]
async fn formula_exec_runc_output() {
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
					}
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

	let result = run_formula_collect_output(formula_and_context, &context)
		.await
		.unwrap();

	assert_eq!(result.exit_code, Some(0));
	let reader = File::open(temp_dir.path().join("output.tar")).unwrap();

	// Unpack output.tar and check contents.
	let mut archive = Archive::new(reader);
	let mut entries = archive.entries().unwrap();
	let mut entry = entries.next().unwrap().unwrap();

	assert_eq!(entry.path().unwrap(), PathBuf::from("test.txt"));
	let mut content = String::new();
	entry.read_to_string(&mut content).unwrap();
	assert_eq!(content, "hello, warpforge!\n");
	assert!(entries.next().is_none());
}
