use std::{env::args, time::Duration};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::{sync::mpsc::Receiver, time::interval};

use crate::{Message, Serializable};

pub(crate) struct TerminalRenderer {
	multi_progress: MultiProgress,
	_prompt: ProgressBar,
	upper_bar: ProgressBar,
	lower_bar: ProgressBar,

	channel: Receiver<Message>,
}

impl TerminalRenderer {
	pub(crate) fn start(channel: Receiver<Message>) {
		let multi_progress = MultiProgress::new();

		let prompt = multi_progress.add(
			ProgressBar::new(1)
				.with_style(
					ProgressStyle::with_template("{prefix:.green} {msg}")
						.expect("invalid indicatif template"),
				)
				.with_prefix("$")
				.with_message(args().collect::<Vec<_>>().join(" ")),
		);
		prompt.tick();

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
				_prompt: prompt,
				upper_bar,
				lower_bar,
				channel,
			}
			.run()
			.await
		});
	}

	async fn run(mut self) {
		let mut interval = interval(Duration::from_secs(1));
		loop {
			let message = tokio::select! {
				message = self.channel.recv() => message,
				_ = interval.tick() => {
					// Make progress bars redraw at least every second,
					// so elapsed time is rendered correctly.
					self.upper_bar.tick();
					self.lower_bar.tick();
					continue;
				}
			};

			let Some(message) = message else {
				break; // Stop rendering, after all `Sender` instances have been destroyed.
			};
			match message {
				Message::Serializable(message) => match message {
					Serializable::Log(message) => {
						self.multi_progress.suspend(|| print!("{}", message))
					}
					Serializable::SetUpper(message) => self.upper_bar.set_message(message),
					Serializable::SetLower(message) => self.lower_bar.set_message(message),
					Serializable::SetUpperPosition(position) => {
						if self.upper_bar.position() != position {
							self.upper_bar.set_position(position);
							self.lower_bar.reset_elapsed();
						}
					}
					Serializable::SetLowerPosition(position) => {
						self.lower_bar.set_position(position)
					}
					Serializable::SetUpperMax(max) => self.upper_bar.set_length(max),
					Serializable::SetLowerMax(max) => self.lower_bar.set_length(max),
				},
				Message::CloseLocalRenderer(notify) => {
					let _ = notify.send(()); // Ignore if no notification could be sent.
					break;
				}
			}
		}
	}
}
