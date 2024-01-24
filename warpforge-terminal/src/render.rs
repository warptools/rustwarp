use std::env::args;

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::sync::mpsc::Receiver;

use crate::Message;

pub(crate) struct TerminalRenderer {
	multi_progress: MultiProgress,
	upper_bar: ProgressBar,
	lower_bar: ProgressBar,

	channel: Receiver<Message>,
}

impl TerminalRenderer {
	pub(crate) fn start(channel: Receiver<Message>) {
		let multi_progress = MultiProgress::new();

		multi_progress.add(
			ProgressBar::new(1)
				.with_style(
					ProgressStyle::with_template("{prefix:.green} {msg}")
						.expect("invalid indicatif template"),
				)
				.with_prefix("$")
				.with_message(args().collect::<Vec<_>>().join(" ")),
		);

		let style = ProgressStyle::with_template(
			"[{elapsed_precise}] [{bar:30.green}] {pos:>3}/{len:3} {msg}",
		)
		.expect("invalid indicatif template")
		.progress_chars("##-");

		let upper_bar = multi_progress.add(ProgressBar::new(1).with_style(style.clone()));
		let lower_bar = multi_progress.add(ProgressBar::new(1).with_style(style));

		tokio::spawn(async move {
			Self {
				multi_progress,
				upper_bar,
				lower_bar,
				channel,
			}
			.run()
			.await
		});
	}

	async fn run(mut self) {
		loop {
			let Some(message) = self.channel.recv().await else { break; };
			match message {
				Message::Log(message) => self.multi_progress.suspend(|| print!("{}", message)),
				Message::SetUpper(message) => self.upper_bar.set_message(message),
				Message::SetLower(message) => self.lower_bar.set_message(message),
				Message::SetUpperPosition(position) => self.upper_bar.set_position(position),
				Message::SetLowerPosition(position) => self.lower_bar.set_position(position),
				Message::SetUpperMax(max) => self.upper_bar.set_length(max),
				Message::SetLowerMax(max) => self.lower_bar.set_length(max),
			}
		}
	}
}
