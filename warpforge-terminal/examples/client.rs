use std::{net::Ipv4Addr, time::Duration};

use tokio::time::sleep;
use warpforge_terminal::client::render_remote_logs;

#[tokio::main]
async fn main() {
	render_remote_logs((Ipv4Addr::LOCALHOST, 8050))
		.await
		.unwrap();
	loop {
		sleep(Duration::MAX).await;
	}
}
