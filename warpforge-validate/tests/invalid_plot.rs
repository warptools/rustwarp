pub mod common;
use common::check_plot;

#[test]
fn invalid_port() {
	let plot = r#"
		{
			"plot.v1": {
				"inputs": {
					"$MSG": "literal:hello, warpforge!"
				},
				"steps": {
					"build": {
						"protoformula": {
							"inputs": {
								"/": "oci:docker.io/busybox:latest",
								<invalid_port>"relative/path"</invalid_port>: "mount:ro:/host/path",
								<invalid_port>"OTHER_MSG"</invalid_port>: "pipe::$MSG"
							},
							"action": {
								"script": {
									"interpreter": "/bin/sh",
									"contents": ["echo $OTHER_MSG"]
								}
							},
							"outputs": {}
						}
					}
				},
				"outputs": {}
			}
		}
	"#;
	check_plot(plot);
}

#[test]
fn invalid_mount() {
	let plot = r#"
		{
			"plot.v1": {
				"inputs": {
					"name0": <invalid_mount>"mount"</invalid_mount>,
					"name1": <invalid_mount>"mount:ro"</invalid_mount>,
					"name2": <invalid_mount>"mount:invalid:/host/path"</invalid_mount>,
					"name3": "mount:ro:/host/path"
				},
				"steps": {},
				"outputs": {}
			}
		}
	"#;
	check_plot(plot);
}

#[test]
fn invalid_literal() {
	let plot = r#"
		{
			"plot.v1": {
				"inputs": {
					"name0": <invalid_mount>"literal"</invalid_mount>,
					"name1": "literal:",
					"name2": "literal:some string"
				},
				"steps": {},
				"outputs": {}
			}
		}
	"#;
	check_plot(plot);
}

#[test]
fn invalid_oci() {
	let plot = r#"
		{
			"plot.v1": {
				"inputs": {
					"name0": <invalid_mount>"oci"</invalid_mount>,
					"name1": <invalid_mount>"oci:??"</invalid_mount>,
					"name2": "oci:docker.io/busybox:latest",
					"name3": "oci:docker.io/alpine:latest"
				},
				"steps": {},
				"outputs": {}
			}
		}
	"#;
	check_plot(plot);
}
