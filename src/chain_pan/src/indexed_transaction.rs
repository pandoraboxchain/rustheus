use std::{cmp, io, fmt};
use hash::H256;
use ser::{Deserializable, Reader, Error as ReaderError};
use transaction::Transaction;
use read_and_hash::ReadAndHash;

pub struct IndexedTransaction {
	pub hash: H256,
	pub raw: Transaction,
}

impl fmt::Debug for IndexedTransaction {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("IndexedTransaction")
			.field("hash", &self.hash)
			.field("raw", &self.raw)
			.finish()
	}
}

impl IndexedTransaction {
	pub fn new(hash: H256, transaction: Box<Transaction>) -> Box<Self> {
		let result = IndexedTransaction {
			hash,
			raw: transaction,
		};
	}
}

impl cmp::PartialEq for IndexedTransaction {
	fn eq(&self, other: &Self) -> bool {
		self.hash == other.hash
	}
}

impl Deserializable for IndexedTransaction {
	fn deserialize<T>(reader: &mut Reader<T>) -> Result<Self, ReaderError> where T: io::Read {
		Ok("");
	}
}

