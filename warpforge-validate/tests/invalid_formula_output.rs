pub mod common;
use common::check_formula;

#[test]
fn missing_from() {
	let formula = r#"
		{
			"formula": {
				"formula.v1": {
					"inputs": <missing_root>{
						"$MSG": "literal:hello",
						"/path": "mount:ro:/host/path"
					}</missing_root>,
					"action": "echo",
					"outputs": {
						"output": <missing_from>{
						}</missing_from>
					}
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
fn invalid_from() {
	let formula = r#"
		{
			"formula": {
				"formula.v1": {
					"inputs": <missing_root>{
						"$MSG": "literal:hello",
						"/path": "mount:ro:/host/path"
					}</missing_root>,
					"action": "echo",
					"outputs": {
						"output.tgz": {
							"from": <invalid_from>"out"</invalid_from>,
							"packtype": "tgz"
						}
					}
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
fn invalid_packtype() {
	let formula = r#"
		{
			"formula": {
				"formula.v1": {
					"inputs": <missing_root>{
						"$MSG": "literal:hello",
						"/path": "mount:ro:/host/path"
					}</missing_root>,
					"action": "echo",
					"outputs": {
						"output": {
							"from": "/out",
							"packtype": <invalid_packtype>"invalid"</invalid_packtype>
						}
					}
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
