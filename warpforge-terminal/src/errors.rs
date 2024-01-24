pub type Result<T> = std::result::Result<T, Error>;

#[derive(thiserror::Error, Debug)]
pub enum Error {
	#[error("internal channel error")]
	ChannelInternal,
}
