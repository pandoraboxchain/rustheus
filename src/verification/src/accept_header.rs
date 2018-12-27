use params::ConsensusParams;
use db::BlockHeaderProvider;
use canon::CanonHeader;
use error::Error;
use work::work_required;
use timestamp::median_timestamp;

pub struct HeaderAcceptor<'a> {
	pub work: HeaderWork<'a>,
	pub median_timestamp: HeaderMedianTimestamp<'a>,
}

impl<'a> HeaderAcceptor<'a> {
	pub fn new(
		store: &'a BlockHeaderProvider,
		consensus: &'a ConsensusParams,
		header: CanonHeader<'a>,
		height: u32,
	) -> Self {
		HeaderAcceptor {
			work: HeaderWork::new(header, store, height, consensus),
			median_timestamp: HeaderMedianTimestamp::new(header, store),
		}
	}

	pub fn check(&self) -> Result<(), Error> {
		try!(self.work.check());
		try!(self.median_timestamp.check());
		Ok(())
	}
}

pub struct HeaderWork<'a> {
	header: CanonHeader<'a>,
	store: &'a BlockHeaderProvider,
	height: u32,
	consensus: &'a ConsensusParams,
}

impl<'a> HeaderWork<'a> {
	fn new(header: CanonHeader<'a>, store: &'a BlockHeaderProvider, height: u32, consensus: &'a ConsensusParams) -> Self {
		HeaderWork {
			header: header,
			store: store,
			height: height,
			consensus: consensus,
		}
	}

	fn check(&self) -> Result<(), Error> {
		let previous_header_hash = self.header.raw.previous_header_hash[0].clone();
		let time = self.header.raw.time;
		let work = work_required(previous_header_hash, time, self.height, self.store, self.consensus);
		if work == self.header.raw.bits {
			Ok(())
		} else {
			Err(Error::Difficulty { expected: work, actual: self.header.raw.bits })
		}
	}
}

pub struct HeaderMedianTimestamp<'a> {
	header: CanonHeader<'a>,
	store: &'a BlockHeaderProvider,
}

impl<'a> HeaderMedianTimestamp<'a> {
	fn new(header: CanonHeader<'a>, store: &'a BlockHeaderProvider) -> Self {
		HeaderMedianTimestamp {
			header: header,
			store: store
		}
	}

	fn check(&self) -> Result<(), Error> {
		if self.header.raw.time <= median_timestamp(&self.header.raw, self.store) {
			Err(Error::Timestamp)
		} else {
			Ok(())
		}
	}
}
