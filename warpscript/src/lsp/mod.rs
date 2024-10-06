use lsp_server::Connection as LspConnection;
use lsp_types::request::Request;

pub fn main_loop(connection: &LspConnection) -> Result<(), lsp_server::ProtocolError> {
	let _ = initialization_handshake(connection);
	for msg in &connection.receiver {
		match msg {
			lsp_server::Message::Request(req) => {
				if connection.handle_shutdown(&req)? {
					return Ok(());
				}
				let id = req.id.clone();
				// I am staggered that lsp_types doesn't seem to offer this switch alredy.  (Am I blind?)
				let resp_value: serde_json::Value = match req.method.as_str() {
					lsp_types::request::HoverRequest::METHOD => {
						let req2 = cast_request::<lsp_types::request::HoverRequest>(req);
						// TODO: we finally got as far as a typed piece of data... real logic could occur now.
						let resp = lsp_types::Hover {
							contents: lsp_types::HoverContents::Scalar(
								lsp_types::MarkedString::String("please".to_owned()),
							),
							range: None,
						};
						serde_json::to_value(resp).expect("response to json")
					}
					_ => todo!("unexpected request method"),
				};
				let response = lsp_server::Response {
					id,
					error: None,
					result: Some(resp_value),
				};
				connection
					.sender
					.send(lsp_server::Message::Response(response))
					.expect("channel send LSP response")
			}
			lsp_server::Message::Notification(x) => todo!("nyi"),
			lsp_server::Message::Response(_) => {
				todo!("didn't expect any lsp responses")
			}
		}
	}
	Ok(())
}

fn cast_request<R>(request: lsp_server::Request) -> R::Params
where
	R: lsp_types::request::Request,
	R::Params: serde::de::DeserializeOwned,
{
	let (_, params) = request.extract(R::METHOD).expect("cast request");
	params
}

fn cast_notification<N>(notification: lsp_server::Notification) -> N::Params
where
	N: lsp_types::notification::Notification,
	N::Params: serde::de::DeserializeOwned,
{
	notification
		.extract::<N::Params>(N::METHOD)
		.expect("cast notification")
}

// Creates lsp_types::ServerCapabilities, sends it to the client,
// and processes the InitializationParams that end up negociated.
//
// TODO: half the features claimed in here are probably currently lies,
// but struct initialization demands all of them, so, okay then.
fn initialization_handshake(connection: &LspConnection) -> lsp_types::InitializeParams {
	let server_capabilities = lsp_types::ServerCapabilities {
		text_document_sync: Some(lsp_types::TextDocumentSyncCapability::Options(
			lsp_types::TextDocumentSyncOptions {
				open_close: Some(true),
				change: Some(lsp_types::TextDocumentSyncKind::FULL),
				will_save: None,
				will_save_wait_until: None,
				save: Some(lsp_types::TextDocumentSyncSaveOptions::SaveOptions(
					lsp_types::SaveOptions {
						include_text: Some(false),
					},
				)),
			},
		)),
		selection_range_provider: None,
		hover_provider: Some(lsp_types::HoverProviderCapability::Simple(true)),
		completion_provider: Some(lsp_types::CompletionOptions {
			resolve_provider: None,
			trigger_characters: Some(vec![".".into()]),
			all_commit_characters: None,
			work_done_progress_options: lsp_types::WorkDoneProgressOptions {
				work_done_progress: None,
			},
			completion_item: None,
		}),
		signature_help_provider: Some(lsp_types::SignatureHelpOptions {
			trigger_characters: Some(vec!["(".into(), ",".into(), ":".into()]),
			retrigger_characters: None,
			work_done_progress_options: lsp_types::WorkDoneProgressOptions {
				work_done_progress: None,
			},
		}),
		definition_provider: Some(lsp_types::OneOf::Left(true)),
		type_definition_provider: None,
		implementation_provider: None,
		references_provider: None,
		document_highlight_provider: None,
		document_symbol_provider: Some(lsp_types::OneOf::Left(true)),
		workspace_symbol_provider: None,
		code_action_provider: Some(lsp_types::CodeActionProviderCapability::Simple(true)),
		code_lens_provider: None,
		document_formatting_provider: Some(lsp_types::OneOf::Left(true)),
		document_range_formatting_provider: None,
		document_on_type_formatting_provider: None,
		rename_provider: None,
		document_link_provider: None,
		color_provider: None,
		folding_range_provider: None,
		declaration_provider: None,
		execute_command_provider: None,
		workspace: None,
		call_hierarchy_provider: None,
		semantic_tokens_provider: None,
		moniker_provider: None,
		linked_editing_range_provider: None,
		experimental: None,
		position_encoding: None,
		inline_value_provider: None,
		inlay_hint_provider: None,
		diagnostic_provider: None,
	};
	let server_capabilities_json =
		serde_json::to_value(server_capabilities).expect("server_capabilities_serde");
	let initialize_params_json = connection
		.initialize(server_capabilities_json)
		.expect("LSP initialize");
	let initialize_params: lsp_types::InitializeParams =
		serde_json::from_value(initialize_params_json).expect("LSP InitializeParams from json");
	initialize_params
}
