use std::fs;

use serde_json::json;
use tempfile::TempDir;
use warpforge_api::formula::FormulaAndContext;

use crate::tests::{default_context, run_formula_collect_output};

#[test]
fn overlayfs_mount() {
	let temp_dir = TempDir::new().unwrap();
	let overlay_dir = temp_dir.path().join("overlay_lower");
	let output_dir = temp_dir.path().join("rw");

	fs::create_dir(&overlay_dir).unwrap();
	fs::create_dir(&output_dir).unwrap();

	let contents = [
		("file.txt", "Hello Warpforge"),
		("ls.sh", "#!/bin/sh\n\nls /container/overlay\n"),
	];
	for (name, content) in contents {
		fs::write(overlay_dir.join(name), content).unwrap();
	}

	let formula_and_context: FormulaAndContext = serde_json::from_value(json!({
		"formula": {
			"formula.v1": {
				"inputs": {
					"/": "oci:docker.io/library/busybox@sha256:22f27168517de1f58dae0ad51eacf1527e7e7ccc47512d3946f56bdbe913f564",
					"/container/overlay": format!("mount:overlay:{}", overlay_dir.to_str().unwrap()),
					"/container/output": format!("mount:rw:{}", output_dir.to_str().unwrap()),
				},
				"action": {
					"script": {
						"interpreter": "/bin/sh",
						"contents": [
							"chmod +x /container/overlay/ls.sh",
							// Read and write in overlay mount.
							"/container/overlay/ls.sh > /container/overlay/file.txt",
							// Using the rw mount ro check if written changes persisted.
							"cp /container/overlay/file.txt /container/output",
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

	let result = run_formula_collect_output(formula_and_context, &default_context()).unwrap();

	assert_eq!(result.exit_code, Some(0));
	for (name, content) in contents {
		assert_eq!(
			fs::read_to_string(overlay_dir.join(name)).unwrap(),
			content,
			"contents of overlayfs lowedir should not change"
		);
	}

	assert_eq!(
		fs::read_to_string(output_dir.join("file.txt"))
			.unwrap()
			.trim(),
		"file.txt\nls.sh",
		"file.txt in output should be modified version"
	);
}
