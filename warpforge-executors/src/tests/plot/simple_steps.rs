use serde_json::json;
use tempfile::TempDir;
use warpforge_api::plot::PlotCapsule;

use crate::{context::Context, plot::run_plot, tests::default_context, Output};

#[tokio::test]
async fn plot_simple_steps() {
	let plot: PlotCapsule = serde_json::from_value(json!({
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

	let temp_dir = TempDir::new().unwrap();
	let context = Context {
		output_path: Some(temp_dir.path().to_owned()),
		..default_context()
	};

	let outputs = run_plot(plot, &context).await.unwrap();

	assert_eq!(outputs, vec![Output{
		name: "output.tar".into(),
		digest: crate::Digest::Sha384("bd00d1ecdaa6988962460b5288953ba8c504f876bd2134b95aa3ef3df993f7fbc6be147898fc94b5f5cff476584d0fd4".into()),
	}]);
}

#[tokio::test]
async fn plot_simple_steps_magled_order() {
	let plot: PlotCapsule = serde_json::from_value(json!({
		"plot.v1": {
			"image": {
				"reference": "docker.io/busybox:latest",
				"readonly": true
			},
			"inputs": {},
			"steps": {
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
			},
			"outputs": {
				"output.tar": "pipe:output:out"
			}
		}
	}))
	.unwrap();

	let temp_dir = TempDir::new().unwrap();
	let context = Context {
		output_path: Some(temp_dir.path().to_owned()),
		..default_context()
	};

	let outputs = run_plot(plot, &context).await.unwrap();

	assert_eq!(outputs, vec![Output{
		name: "output.tar".into(),
		digest: crate::Digest::Sha384("bd00d1ecdaa6988962460b5288953ba8c504f876bd2134b95aa3ef3df993f7fbc6be147898fc94b5f5cff476584d0fd4".into()),
	}]);
}
