use std::{cmp, io, fmt};
use hash::H256;
use ser::{Deserializable, Reader, Error as ReaderError};
use payment_transaction::PaymentTransaction;
use read_and_hash::ReadAndHash;

//TODO create separeted impl for pan tx,s

#[derive(Default, Clone)]
pub struct IndexedTransaction {
	pub hash: H256,
	pub raw: PaymentTransaction,
}

impl fmt::Debug for IndexedTransaction {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("IndexedTransaction")
			.field("hash", &self.hash)
			.field("raw", &self.raw)
			.finish()
	}
}

impl<T> From<T> for IndexedTransaction where PaymentTransaction: From<T> {
	fn from(other: T) -> Self {
		let tx = PaymentTransaction::from(other);
		IndexedTransaction {
			hash: tx.hash(),
			raw: tx,
		}
	}
}

impl IndexedTransaction {
	pub fn new(hash: H256, transaction: PaymentTransaction) -> Self {
		IndexedTransaction {
			hash,
			raw: transaction,
		}
	}
}

impl cmp::PartialEq for IndexedTransaction {
	fn eq(&self, other: &Self) -> bool {
		self.hash == other.hash
	}
}

impl Deserializable for IndexedTransaction {
	fn deserialize<T>(reader: &mut Reader<T>) -> Result<Self, ReaderError> where T: io::Read {
		let data = try!(reader.read_and_hash::<PaymentTransaction>());
		// TODO: use len
		let tx = IndexedTransaction {
			raw: data.data,
			hash: data.hash,
		};

		Ok(tx)
	}
}
