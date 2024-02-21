use crate::Message;

pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("internal channel error")]
	ChannelInternal { input: Message },
}

#[derive(thiserror::Error, Debug)]
#[error("global logger has already been set and cannot be redefined")]
pub struct GlobalLoggerAlreadyDefined;
