use serde_json::json;
use warpforge_api::formula::FormulaAndContext;

use crate::test::{run_formula_collect_output, RunOutputLine};

#[tokio::test]
async fn formula_exec_runc_it_works() {
	let formula_and_context: FormulaAndContext = serde_json::from_value(json!({
		"formula": {
			"formula.v1": {
				"image": {
					"reference": "docker.io/busybox:latest",
					"readonly": true,
				},
				"inputs": {
					"$MSG": "literal:hello from warpforge!",
				},
				"action": {
					"exec": {
						"command": [
							"/bin/sh",
							"-c",
							"echo $MSG",
						]
					}
				},
				"outputs": {},
			}
		},
		"context": {
			"context.v1": {
				"warehouses": {}
			}
		}
	}))
	.expect("failed to parse formula json");

	let result = run_formula_collect_output(formula_and_context)
		.await
		.unwrap();

	assert_eq!(result.exit_code, Some(0));
	assert_eq!(
		result.outputs,
		vec![RunOutputLine {
			channel: 1,
			line: "hello from warpforge!".into(),
		}],
	);
}

#[tokio::test]
async fn formula_script_runc_it_works() {
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
						"MESSAGE='hello, this is a script action'",
						"echo $MESSAGE",
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

	let result = run_formula_collect_output(formula_and_context)
		.await
		.unwrap();

	assert_eq!(result.exit_code, Some(0));
	assert_eq!(
		result.outputs,
		vec![RunOutputLine {
			channel: 1,
			line: "hello, this is a script action".into(),
		}],
	);
}
