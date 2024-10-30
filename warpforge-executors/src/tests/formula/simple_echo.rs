use serde_json::json;
use warpforge_api::formula::FormulaAndContext;

use crate::tests::{default_context, run_formula_collect_output, RunOutputLine};

#[test]
fn formula_exec_runc_it_works() {
	let formula_and_context: FormulaAndContext = serde_json::from_value(json!({
		"formula": {
			"formula.v1": {
				"inputs": {
					"/": "oci:docker.io/library/busybox@sha256:22f27168517de1f58dae0ad51eacf1527e7e7ccc47512d3946f56bdbe913f564",
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

	let result = run_formula_collect_output(formula_and_context, &default_context()).unwrap();

	assert_eq!(result.exit_code, Some(0));
	assert_eq!(
		result.console,
		vec![RunOutputLine {
			channel: 1,
			line: "hello from warpforge!".into(),
		}],
	);
}

#[test]
fn formula_script_runc_it_works() {
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

	let result = run_formula_collect_output(formula_and_context, &default_context()).unwrap();

	assert_eq!(result.exit_code, Some(0));
	assert_eq!(
		result.console,
		vec![RunOutputLine {
			channel: 1,
			line: "hello, this is a script action".into(),
		}],
	);
}
