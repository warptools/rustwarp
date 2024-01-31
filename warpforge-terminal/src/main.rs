use std::net::Ipv4Addr;

use tokio::signal;
use warpforge_terminal::client::render_remote_logs;

#[tokio::main]
async fn main() {
	let token = render_remote_logs((Ipv4Addr::LOCALHOST, 8050))
		.await
		.unwrap();
	signal::ctrl_c().await.unwrap();
	token.cancel();
}
