mod lsp;

pub fn main() -> Result<(), lsp_server::ProtocolError> {
	eprintln!("Hei!  This is an LSP server.  It's not meant to be run by humans.");

	// Create the transport over stdio.
	let (connection, io_threads) = lsp_server::Connection::stdio();

	// Run the server and wait it to return.
	// Typically there is an LSP Exit event that triggers this.
	lsp::main_loop(&connection)?;

	// Shut down gracefully.
	drop(connection);
	io_threads.join().expect("joining lsp threads");

	Ok(())
}
