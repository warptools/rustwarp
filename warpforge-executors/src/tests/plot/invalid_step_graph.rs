use serde_json::json;
use warpforge_api::plot::PlotCapsule;

use crate::plot::PlotGraph;

#[test]
fn cyclic_graph() {
	let PlotCapsule::V1(plot) = serde_json::from_value(json!({
		"plot.v1": {
			"image": {
				"reference": "docker.io/busybox:latest",
				"readonly": true
			},
			"inputs": {},
			"steps": {
				"create": {
					"protoformula": {
						"inputs": {
							"/in": "pipe:output:out"},
						"action": {
							"script": {
								"interpreter": "/bin/sh",
								"contents": [
									"echo \"hello, plot!\" > /out/test.txt"
								]
							}
						},
						"outputs": {
							"out": { "from": "/out" }
						}
					}
				},
				"copy": {
					"protoformula": {
						"inputs": {
							"/in": "pipe:create:out"
						},
						"action": {
							"script": {
								"interpreter": "/bin/sh",
								"contents": [
									"cp /in/test.txt /out"
								]
							}
						},
						"outputs": {
							"copied": { "from": "/out" }
						}
					}
				},
				"output": {
					"protoformula": {
						"inputs": {
							"/in": "pipe:copy:copied"
						},
						"action": {
							"exec": {
								"command": ["/bin/cp", "-R", "/in", "/out"]
							}
						},
						"outputs": {
							"out": { "from": "/out" }
						}
					}
				}
			},
			"outputs": {
				"output.tar": "pipe:output:out"
			}
		}
	}))
	.unwrap();

	let graph = PlotGraph::new(&plot);
	assert!(graph.validate().is_err());
	assert!(graph.validate_no_cycles().is_err());
}

#[test]
fn invalid_edges() {
	let PlotCapsule::V1(plot) = serde_json::from_value(json!({
		"plot.v1": {
			"image": {
				"reference": "docker.io/busybox:latest",
				"readonly": true
			},
			"inputs": {},
			"steps": {
				"create": {
					"protoformula": {
						"inputs": {},
						"action": {
							"script": {
								"interpreter": "/bin/sh",
								"contents": [
									"echo \"hello, plot!\" > /out/test.txt"
								]
							}
						},
						"outputs": {
							"out": { "from": "/out" }
						}
					}
				},
				"copy": {
					"protoformula": {
						"inputs": {
							"/in": "pipe:create:out",
							"/in": "pipe:invalid:out"
						},
						"action": {
							"script": {
								"interpreter": "/bin/sh",
								"contents": [
									"cp /in/test.txt /out"
								]
							}
						},
						"outputs": {
							"copied": { "from": "/out" }
						}
					}
				},
				"output": {
					"protoformula": {
						"inputs": {
							"/in": "pipe:copy:copied",
							"/in": "pipe:invalid:out"
						},
						"action": {
							"exec": {
								"command": ["/bin/cp", "-R", "/in", "/out"]
							}
						},
						"outputs": {
							"out": { "from": "/out" }
						}
					}
				}
			},
			"outputs": {
				"output.tar": "pipe:output:out"
			}
		}
	}))
	.unwrap();

	let graph = PlotGraph::new(&plot);
	assert!(graph.validate().is_err());
	assert!(graph.validate_dependencies_exist().is_err());
}
