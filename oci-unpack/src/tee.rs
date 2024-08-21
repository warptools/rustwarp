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
pub trait WriteExt: Write {
	fn tee<R: Write>(self, other: R) -> TeeWriter<Self, R>
	where
		Self: Sized;
}

impl<W: Write> WriteExt for W {
	fn tee<R: Write>(self, other: R) -> TeeWriter<Self, R>
	where
		Self: Sized,
	{
		TeeWriter::new(self, other)
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

/// A writer which writes its input to two writers.
pub struct TeeWriter<L, R> {
	left: L,
	right: R,
}

impl<L: Write, R: Write> TeeWriter<L, R> {
	pub fn new(left: L, right: R) -> Self {
		Self { left, right }
	}
}

impl<L: Write, R: Write> Write for TeeWriter<L, R> {
	fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
		let len = self.left.write(buf)?;
		self.right.write_all(&buf[..len])?;
		Ok(len)
	}

	fn flush(&mut self) -> std::io::Result<()> {
		self.left.flush().and(self.right.flush())
	}

	fn write_all(&mut self, buf: &[u8]) -> std::io::Result<()> {
		self.left.write_all(buf)?;
		self.right.write_all(buf)?;
		Ok(())
	}
}
