use std::{collections::HashMap, env::args, thread, time::Duration};

use crossbeam_channel::Receiver;
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

use crate::{BarId, Message, Serializable};

pub(crate) struct TerminalRenderer {
	multi_progress: Option<MultiProgress>,
	prompt: Option<ProgressBar>,
	bars: HashMap<BarId, ProgressBar>,

	channel: Receiver<Message>,
}

impl TerminalRenderer {
	pub(crate) fn start(channel: Receiver<Message>) {
		thread::spawn(move || {
			Self {
				multi_progress: None,
				prompt: None,
				bars: HashMap::new(),
				channel,
			}
			.run()
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

	fn run(mut self) {
		loop {
			let timeout = Duration::from_secs(1);
			let message = match self.channel.recv_timeout(timeout) {
				Ok(message) => message,
				Err(err) => {
					if err.is_timeout() {
						// Make progress bars redraw at least every second,
						// so elapsed time is rendered correctly.
						for bar in self.bars.values() {
							bar.tick();
						}
						continue;
					} else {
						debug_assert!(err.is_disconnected());
						break;
					}
				}
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
						self.add_multiprogress();
						match message {
							Serializable::Log(_) => unreachable!(),
							Serializable::CreateBar { id, max } => {
								let multi = self.multi_progress.as_ref().unwrap();
								let style = ProgressStyle::with_template(
									"[{elapsed_precise}] [{bar:30.green}] {pos:>3}/{len:3} {msg}",
								)
								.expect("invalid indicatif template")
								.progress_chars("##-");
								let bar = multi.add(ProgressBar::new(max).with_style(style));

								self.bars.insert(id, bar);
							}
							Serializable::RemoveBar(id) => {
								self.bars.remove(&id);
							}
							Serializable::SetBarText(id, text) => {
								if let Some(bar) = self.bars.get(&id) {
									bar.set_message(text);
								}
							}
							Serializable::SetBarPosition(id, position) => {
								if let Some(bar) = self.bars.get(&id) {
									bar.set_position(position);
								}
							}
							Serializable::SetBarMax(id, max) => {
								if let Some(bar) = self.bars.get(&id) {
									bar.set_length(max);
								}
							}
						}
					}
				}
			}
		}
	}
}
