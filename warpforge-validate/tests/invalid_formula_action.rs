mod common;
use common::check_formula;

#[test]
fn no_action() {
	let formula = r#"
		{
			"formula": {
				"formula.v1": {
					"inputs": <missing_root>{
						"$MSG": "literal:hello",
						"/path": "mount:ro:/host/path"
					}</missing_root>,
					"action": <no_action>{}</no_action>,
					"outputs": {}
				}
			},
			"context": {
				"context.v1": {
					"warehouses": {}
				}
			}
		}
	"#;
	check_formula(formula);
}

#[test]
fn multiple_actions() {
	let formula = r#"
		{
			"formula": {
				"formula.v1": {
					"inputs": <missing_root>{
						"$MSG": "literal:hello",
						"/path": "mount:ro:/host/path"
					}</missing_root>,
					"action": <multiple_actions>{
						"exec": {
							"command": ["echo", "hello"]
						},
						"script": {
							"interpreter": "/bin/sh",
							"contents": ["echo hello"]
						}
					}</multiple_actions>,
					"outputs": {}
				}
			},
			"context": {
				"context.v1": {
					"warehouses": {}
				}
			}
		}
	"#;
	check_formula(formula);
}

#[test]
fn invalid_action() {
	let formula = r#"
		{
			"formula": {
				"formula.v1": {
					"inputs": <missing_root>{
						"$MSG": "literal:hello",
						"/path": "mount:ro:/host/path"
					}</missing_root>,
					"action": {
						<invalid_action>"invalid"</invalid_action>: {
							"command": ["echo", "hello"]
						}
					},
					"outputs": {}
				}
			},
			"context": {
				"context.v1": {
					"warehouses": {}
				}
			}
		}
	"#;
	check_formula(formula);
}

#[test]
fn missing_command() {
	let formula = r#"
		{
			"formula": {
				"formula.v1": {
					"inputs": <missing_root>{
						"$MSG": "literal:hello",
						"/path": "mount:ro:/host/path"
					}</missing_root>,
					"action": {
						"exec": <missing_command>{
						}</missing_command>
					},
					"outputs": {}
				}
			},
			"context": {
				"context.v1": {
					"warehouses": {}
				}
			}
		}
	"#;
	check_formula(formula);
}

#[test]
fn command_not_string() {
	let formula = r#"
		{
			"formula": {
				"formula.v1": {
					"inputs": <missing_root>{
						"$MSG": "literal:hello",
						"/path": "mount:ro:/host/path"
					}</missing_root>,
					"action": {
						"exec": {
							"command": ["echo", "hello", <command_not_string>5</command_not_string>]
						}
					},
					"outputs": {}
				}
			},
			"context": {
				"context.v1": {
					"warehouses": {}
				}
			}
		}
	"#;
	check_formula(formula);
}
