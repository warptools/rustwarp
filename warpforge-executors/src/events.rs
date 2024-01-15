/// Event is the type used to shuttle infomation produced by subprocesses.
/// It contains either bytes from stdout, from stderr, or an exit code.
///
/// In most usages, we buffer these to a full line before sending an event.
/// In those cases, the linebreak byte will still be attached.
/// Some subprocess modes will send smaller increments.
/// (In practice: when we're running subprocesses for plugins, they generally
/// have line-oriented protocols, e.g. JSONL.  For interactive appearances
/// on containers, however, we need to relay input more or less constantly.)
#[derive(Debug)]
pub struct Event {
	pub topic: String,
	/// Generally, the container ident.
	pub body: EventBody,
}

#[derive(Debug)]
pub enum EventBody {
	Output {
		/// Follows the convention of unix fd's: 1 is stdout, 2 is stderr.
		/// So far we have no use of further numbers.
		channel: i32,
		val: String, // FIXME String is most certainly not the right type here.  Find the right tokio reader system to return either Bytes, Vec<u8>, OsStr, or something sensible like that.
	},
	ExitCode(Option<i32>),
}
