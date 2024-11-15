mod common;
use common::check_formula;

#[test]
fn trailing_comma() {
	let formula = r#"
		{
			"formula": {
				"formula.v1": {
					"inputs": {
						"/": <missing_digest>"oci:busybox"</missing_digest>,
						"/path": "mount:ro:/host/path"<trailing_comma>,</trailing_comma>
					},
					"action": "echo",
					"outputs": {
						"test": {
							"from": "/out",
							"packtype": "tgz"<trailing_comma>,</trailing_comma>
						}<trailing_comma>,</trailing_comma>
					}<trailing_comma>,</trailing_comma>
				}
			},
			"context": {
				"context.v1": {
					"warehouses": {}
				}
			}<trailing_comma>,</trailing_comma>
		}
	"#;
	check_formula(formula);
}

#[test]
fn missing_closing_curly() {
	let formula = r#"
		{
			"formula": {
				"formula.v1": {
					"inputs": {
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
			}<missing_closing_curly></missing_closing_curly> "#;
	check_formula(formula);
}

#[test]
fn invalid_json() {
	let formula = r#"
		{
			<invalid_json></invalid_json>formula: {
				"formula.v1": {
					"inputs": {
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
