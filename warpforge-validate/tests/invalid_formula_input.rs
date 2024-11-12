use common::{check_validation_locations, prepare_input};
use warpforge_validate::validate_formula;

mod common;

#[test]
fn missing_root() {
	let (json, locations) = prepare_input(
		r#"
		{
			"formula": {
				"formula.v1": {
					"inputs": <error>{
						"$MSG": "literal:hello",
						"/path": "mount:ro:/host/path"
					}</error>,
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
	"#,
	);

	let result = validate_formula(&json);
	check_validation_locations(&result, &json, &locations);
}
