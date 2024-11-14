mod common;
use common::check_formula;

#[test]
fn missing_root() {
	let formula = r#"
			{
				"formula": {
					"formula.v1": {
						"inputs": <missing_root>{
							"$MSG": "literal:hello",
							"/path": "mount:ro:/host/path"
						}</missing_root>,
						"action": "echo",
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
fn root_not_oci() {
	let formula = r#"
			{
				"formula": {
					"formula.v1": {
						"inputs": {
							"/": <root_not_oci>"mount:ro:/host/path"</root_not_oci>,
							"$MSG": "literal:hello",
							"/path": "mount:ro:/host/path"
						},
						"action": "echo",
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
fn oci_invalid() {
	let formula = r#"
			{
				"formula": {
					"formula.v1": {
						"inputs": {
							"/": <oci_invalid>"oci:$$"</oci_invalid>,
							"$MSG": "literal:hello",
							"/path": "mount:ro:/host/path"
						},
						"action": "echo",
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
fn oci_without_digest() {
	let formula = r#"
			{
				"formula": {
					"formula.v1": {
						"inputs": {
							"/": <oci_without_digest>"oci:busybox"</oci_without_digest>,
							"$MSG": "literal:hello",
							"/path": "mount:ro:/host/path"
						},
						"action": "echo",
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
fn invalid_port() {
	let formula = r#"
			{
				"formula": {
					"formula.v1": {
						"inputs": <missing_root>{
							<invalid_port>"MSG"</invalid_port>: "literal:hello",
							"/path": "mount:ro:/host/path"
						}</missing_root>,
						"action": "echo",
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
fn invalid_input_value() {
	let formula = r#"
			{
				"formula": {
					"formula.v1": {
						"inputs": <missing_root>{
							"$MSG": <invalid_input_value>"literal"</invalid_input_value>,
							"/path": "mount:ro:/host/path"
						}</missing_root>,
						"action": "echo",
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
fn env_not_literal() {
	let formula = r#"
			{
				"formula": {
					"formula.v1": {
						"inputs": <missing_root>{
							"$MSG": <env_not_literal>"mount:ro:/host/path"</env_not_literal>,
							"/path": "mount:ro:/host/path"
						}</missing_root>,
						"action": "echo",
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
