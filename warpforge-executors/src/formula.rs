#[allow(dead_code)]
pub enum Formula {
	Runc(crate::runc::Executor),
	//Gvisor(crate::gvisor::GvisorExecutor),
}

impl Formula {
	#[allow(dead_code)]
	pub async fn run(
		&self,
		formula_and_context: warpforge_api::formula::FormulaAndContext,
		outbox: tokio::sync::mpsc::Sender<crate::Event>,
	) -> Result<(), crate::Error> {
		use indexmap::IndexMap;
		use warpforge_api::formula;
		let params = crate::ContainerParams {
			action: match formula_and_context.formula {
				formula::FormulaCapsule::V1(f) => f.action,
			},
			mounts: {
				// IndexMap does have a From trait, but I didn't want to copy the destinations manually.
				IndexMap::new()
				// todo: more initializer here
			},
			root_path: "/tmp/rootfs".to_string(),
		};
		match self {
			Formula::Runc(e) => e.run(&params, outbox).await,
			/*Formula::Gvisor(_e) => Err(crate::Error::CatchallCauseless {
				msg: "gvisor exector not implemented".to_string(),
			}),*/
		}
	}
}

#[cfg(test)]
mod tests {
	use super::*;
	use std::path::Path;

	//use expect_test::expect;
	use tokio::sync::mpsc;

	#[tokio::main]
	#[test]
	async fn formula_runc_it_works() {
		let formula_and_context: warpforge_api::formula::FormulaAndContext =
			serde_json::from_str(
				r#"
{
  "formula": {
    "formula.v1": {
      "inputs": {
        "/": "ware:tar:4z9DCTxoKkStqXQRwtf9nimpfQQ36dbndDsAPCQgECfbXt3edanUrsVKCjE9TkX2v9"
      },
      "action": {
        "exec": {
          "command": [
            "/bin/sh",
            "-c",
            "echo hello from warpforge!"
          ]
        }
      },
      "outputs": {}
    }
  },
  "context": {
    "context.v1": {
      "warehouses": {
        "tar:4z9DCTxoKkStqXQRwtf9nimpfQQ36dbndDsAPCQgECfbXt3edanUrsVKCjE9TkX2v9": "https://warpsys.s3.amazonaws.com/warehouse/4z9/DCT/4z9DCTxoKkStqXQRwtf9nimpfQQ36dbndDsAPCQgECfbXt3edanUrsVKCjE9TkX2v9"
      }
    }
  }
}"#,
			).expect("failed to parse formula json");

		let cfg = crate::runc::Executor {
			ersatz_dir: Path::new("/tmp/warpforge-test-executor-runc/run").to_owned(),
			log_file: Path::new("/tmp/warpforge-test-executor-runc/log").to_owned(),
		};
		let (gather_chan, mut gather_chan_recv) = mpsc::channel::<crate::events::Event>(32);

		let executor = Formula::Runc(cfg);

		// empty gather_chan
		let gather_handle = tokio::spawn(async move {
			while let Some(evt) = gather_chan_recv.recv().await {
				match evt.body {
					crate::events::EventBody::Output { .. } => {}
					crate::events::EventBody::ExitCode(code) => {
						assert_eq!(code, Some(0));
						//return; // stop processing events
					}
				};
				println!("event! {:?}", evt);
			}
		});

		executor
			.run(formula_and_context, gather_chan)
			.await
			.expect("it didn't fail");
		gather_handle.await.expect("gathering events failed");
	}
}
