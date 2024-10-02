use std::fs;

use serde_json::json;
use tempfile::TempDir;
use warpforge_api::formula::FormulaAndContext;

use crate::tests::{default_context, run_formula_collect_output};

#[tokio::test]
async fn runc_rbind_mounts() {
	let temp_dir = TempDir::new().unwrap();
	let input_dir = temp_dir.path().join("ro");
	let output_dir = temp_dir.path().join("rw");

	fs::create_dir(&input_dir).unwrap();
	fs::create_dir(&output_dir).unwrap();

	let contents = [
		("file.txt", "Hello Warpforge"),
		("ls_root.sh", "#!/bin/sh\n\nls /\n"),
	];
	for (name, content) in contents {
		fs::write(input_dir.join(name), content).unwrap();
	}

	let formula_and_context: FormulaAndContext = serde_json::from_value(json!({
		"formula": {
			"formula.v1": {
				"image": {
					"reference": "docker.io/busybox:latest",
					"readonly": true,
				},
				"inputs": {
					"/container/input": format!("mount:ro:{}", input_dir.to_str().unwrap()),
					"/container/output": format!("mount:rw:{}", output_dir.to_str().unwrap()),
				},
				"action": {
					"exec": {
						"command": [
							"/bin/sh",
							"-c",
							"cp -R /container/input/* /container/output",
						]
					}
				},
				"outputs": {},
			}
		},
		"context": {
			"context.v1": {
				"warehouses": {},
			}
		}
	}))
	.expect("failed to parse formula json");

	let result = run_formula_collect_output(formula_and_context, &default_context())
		.await
		.unwrap();

	assert_eq!(result.exit_code, Some(0));
	for (name, content) in contents {
		assert_eq!(fs::read_to_string(output_dir.join(name)).unwrap(), content);
	}
}

#[tokio::test]
async fn runc_rbind_mounts_relative_path() {
	let temp_dir = TempDir::new().unwrap();
	let input_dir = temp_dir.path().join("ro_dir");
	let output_dir = temp_dir.path().join("nested").join("rw_dir");

	fs::create_dir(&input_dir).unwrap();
	fs::create_dir_all(&output_dir).unwrap();

	let contents = [
		("info.txt", "here we test relative rbind mounts"),
		("explain.txt", "mount_path: absolute portion"),
	];
	for (name, content) in contents {
		fs::write(input_dir.join(name), content).unwrap();
	}

	let formula_and_context: FormulaAndContext = serde_json::from_value(json!({
		"formula": {
			"formula.v1": {
				"image": {
					"reference": "docker.io/busybox:latest",
					"readonly": true,
				},
				"inputs": {
					"/container/input": "mount:ro:ro_dir",
					"/container/output": "mount:rw:nested/rw_dir",
				},
				"action": {
					"exec": {
						"command": [
							"/bin/sh",
							"-c",
							"cp -R /container/input/* /container/output",
						]
					}
				},
				"outputs": {},
			}
		},
		"context": {
			"context.v1": {
				"warehouses": {},
			}
		}
	}))
	.expect("failed to parse formula json");

	let mut context = default_context();
	context.mount_path = Some(temp_dir.path().to_owned());

	let result = run_formula_collect_output(formula_and_context, &context)
		.await
		.unwrap();

	assert_eq!(result.exit_code, Some(0));
	for (name, content) in contents {
		assert_eq!(fs::read_to_string(output_dir.join(name)).unwrap(), content);
	}
}

#[tokio::test]
async fn runc_cannot_write_to_ro_mount() {
	let temp_dir = TempDir::new().unwrap();
	let input_dir = temp_dir.path().join("ro");
	fs::create_dir(&input_dir).unwrap();

	let formula_and_context: FormulaAndContext = serde_json::from_value(json!({
		"formula": {
			"formula.v1": {
				"image": {
					"reference": "docker.io/busybox:latest",
					"readonly": true,
				},
				"inputs": {
					"/container/mount": format!("mount:ro:{}", input_dir.to_str().unwrap()),
				},
				"action": {
					"exec": {
						"command": [
							"/bin/sh",
							"-c",
							"echo hello > /container/mount/myfile.txt",
						]
					}
				},
				"outputs": {},
			}
		},
		"context": {
			"context.v1": {
				"warehouses": {},
			}
		}
	}))
	.expect("failed to parse formula json");

	let result = run_formula_collect_output(formula_and_context, &default_context())
		.await
		.unwrap();

	assert_ne!(result.exit_code, Some(0));
}
