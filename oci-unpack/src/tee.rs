//! Cherry picked from: https://github.com/TheOnlyMrCat/io_tee
//! (Did not want to add a dependency, which might not be maintained.)

use std::io::{Read, Write};

pub trait ReadExt: Read {
	fn tee<W: Write>(self, out: W) -> TeeReader<Self, W>
	where
		Self: Sized;
}

impl<R: Read> ReadExt for R {
	fn tee<W: Write>(self, out: W) -> TeeReader<Self, W>
	where
		Self: Sized,
	{
		TeeReader::new(self, out)
	}
}

/// A reader which tees its input to another writer.
pub struct TeeReader<R, W> {
	reader: R,
	writer: W,
}

impl<R: Read, W: Write> TeeReader<R, W> {
	pub fn new(reader: R, writer: W) -> Self {
		Self { reader, writer }
	}
}

impl<R: Read, W: Write> Read for TeeReader<R, W> {
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		let len = self.reader.read(buf)?;
		self.writer.write_all(&buf[..len])?;
		Ok(len)
	}

	fn read_to_end(&mut self, buf: &mut Vec<u8>) -> std::io::Result<usize> {
		let start = buf.len();
		let len = self.reader.read_to_end(buf)?;
		self.writer.write_all(&buf[start..start + len])?;
		Ok(len)
	}

	fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
		self.reader.read_exact(buf)?;
		self.writer.write_all(buf)?;
		Ok(())
	}
}
