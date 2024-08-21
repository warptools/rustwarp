use std::{env::args, time::Duration};

use indicatif::{MultiProgress, ProgressBar, ProgressStyle};
use tokio::{sync::mpsc::Receiver, time::interval};

use crate::{Message, Serializable};

pub(crate) struct TerminalRenderer {
	multi_progress: Option<MultiProgress>,
	prompt: Option<ProgressBar>,
	upper_bar: Option<ProgressBar>,
	lower_bar: Option<ProgressBar>,

	channel: Receiver<Message>,
}

impl TerminalRenderer {
	pub(crate) fn start(channel: Receiver<Message>) {
		tokio::spawn(async move {
			Self {
				multi_progress: None,
				prompt: None,
				upper_bar: None,
				lower_bar: None,
				channel,
			}
			.run()
			.await
		});
	}

	#[inline]
	fn add_multiprogress(&mut self) {
		if self.multi_progress.is_some() {
			return;
		}

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

		self.multi_progress = Some(multi_progress);
		self.prompt = Some(prompt);
	}

	#[inline]
	fn style() -> ProgressStyle {
		ProgressStyle::with_template("[{elapsed_precise}] [{bar:30.green}] {pos:>3}/{len:3} {msg}")
			.expect("invalid indicatif template")
			.progress_chars("##-")
	}

	async fn run(mut self) {
		let mut interval = interval(Duration::from_secs(1));
		loop {
			let message = tokio::select! {
				message = self.channel.recv() => message,
				_ = interval.tick() => {
					// Make progress bars redraw at least every second,
					// so elapsed time is rendered correctly.
					if let Some(upper_bar) = &self.upper_bar {
						upper_bar.tick();
					}
					if let Some(lower_bar) = &self.lower_bar {
						lower_bar.tick();
					}
					continue;
				}
			};

			let Some(message) = message else {
				break; // Stop rendering, after all `Sender` instances have been destroyed.
			};
			match message {
				Message::CloseLocalRenderer(notify) => {
					let _ = notify.send(()); // Ignore if no notification could be sent.
					break;
				}
				Message::Serializable(message) => {
					if let Serializable::Log(message) = &message {
						match &self.multi_progress {
							Some(multi_progress) => multi_progress.suspend(|| print!("{message}")),
							None => print!("{message}"),
						}
					} else {
						match &message {
							Serializable::SetUpper(_)
							| Serializable::SetUpperMax(_)
							| Serializable::SetUpperPosition(_) => {
								if self.upper_bar.is_none() {
									self.add_multiprogress();
									self.upper_bar = Some(
										(self.multi_progress.as_ref().unwrap())
											.add(ProgressBar::new(1).with_style(Self::style())),
									);
								}
								let upper_bar = self.upper_bar.as_ref().unwrap();

								match message {
									Serializable::SetUpper(message) => {
										upper_bar.set_message(message);
									}
									Serializable::SetUpperMax(max) => upper_bar.set_length(max),
									Serializable::SetUpperPosition(position) => {
										if upper_bar.position() != position {
											upper_bar.set_position(position);
											if let Some(b) = &self.lower_bar {
												b.reset_elapsed()
											}
										}
									}
									_ => unreachable!(),
								}
							}
							Serializable::SetLower(_)
							| Serializable::SetLowerMax(_)
							| Serializable::SetLowerPosition(_) => {
								if self.lower_bar.is_none() {
									self.add_multiprogress();
									self.lower_bar = Some(
										(self.multi_progress.as_ref().unwrap())
											.add(ProgressBar::new(1).with_style(Self::style())),
									);
								}
								let lower_bar = self.lower_bar.as_ref().unwrap();

								match message {
									Serializable::SetLower(message) => {
										lower_bar.set_message(message);
									}
									Serializable::SetLowerMax(max) => lower_bar.set_length(max),
									Serializable::SetLowerPosition(position) => {
										lower_bar.set_position(position);
									}
									_ => unreachable!(),
								}
							}
							_ => unreachable!(),
						}
					}
				}
			}
		}
	}
}
