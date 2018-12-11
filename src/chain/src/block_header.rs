use std::fmt;
use hex::FromHex;
use ser::{deserialize, serialize};
use crypto::dhash256;
use compact::Compact;
use hash::H256;

#[derive(PartialEq, Clone, Serializable, Deserializable)]
pub struct BlockHeader {
	pub version: u32,
	pub previous_header_hash: Vec<H256>,
	pub merkle_root_hash: H256,
	pub witness_merkle_root_hash: H256,
	pub time: u32,
	pub bits: Compact,
	pub nonce: u32,
}

impl BlockHeader {
	pub fn hash(&self) -> H256 {
		dhash256(&serialize(self))
	}
}

impl fmt::Debug for BlockHeader {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		f.debug_struct("BlockHeader")
			.field("version", &self.version)
			.field("previous_header_hash", &self.previous_header_hash)
			.field("merkle_root_hash", &self.merkle_root_hash)
			.field("witness_merkle_root_hash", &self.witness_merkle_root_hash)
			.field("time", &self.time)
			.field("bits", &self.bits)
			.field("nonce", &self.nonce)
			.finish()
	}
}

impl From<&'static str> for BlockHeader {
	fn from(s: &'static str) -> Self {
		deserialize(&s.from_hex().unwrap() as &[u8]).unwrap()
	}
}

#[cfg(test)]
mod tests {
	use hash::H256;
	use ser::{Reader, Error as ReaderError, Stream};
	use super::BlockHeader;

	#[test]
	fn test_block_header_stream() {
		let previous_header_hash: H256 = [2; 32].into();
		let hashes = vec![previous_header_hash];

		let block_header = BlockHeader {
			version: 1,
			previous_header_hash: hashes,
			merkle_root_hash: [3; 32].into(),
			witness_merkle_root_hash: [4; 32].into(),
			time: 5,
			bits: 6.into(),
			nonce: 7,
		};

		let mut stream = Stream::default();
		stream.append(&block_header);

		let expected = vec![
			1, 0, 0, 0,
			1,
			2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
			3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
			4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
			5, 0, 0, 0,
			6, 0, 0, 0,
			7, 0, 0, 0,
		].into();

		assert_eq!(stream.out(), expected);
	}

	#[test]
	fn test_block_header_reader() {
		let buffer = vec![
			1, 0, 0, 0,
			1,
			2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
			3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
			4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
			5, 0, 0, 0,
			6, 0, 0, 0,
			7, 0, 0, 0,
		];

		let mut reader = Reader::new(&buffer);

		let previous_header_hash: H256 = [2; 32].into();
		let hashes = vec![previous_header_hash];
		let expected = BlockHeader {
			version: 1,
			previous_header_hash: hashes,
			merkle_root_hash: [3; 32].into(),
			witness_merkle_root_hash: [4; 32].into(),
			time: 5,
			bits: 6.into(),
			nonce: 7,
		};

		assert_eq!(expected, reader.read().unwrap());
		assert_eq!(ReaderError::UnexpectedEnd, reader.read::<BlockHeader>().unwrap_err());
	}

	#[test]
	fn test_two_parent_blocks_header_stream() {
		let previous_header_hash_1: H256 = [0; 32].into();
		let previous_header_hash_2: H256 = [1; 32].into();
		let hashes = vec![previous_header_hash_1, previous_header_hash_2];

		let block_header = BlockHeader {
			version: 1,
			previous_header_hash: hashes,
			merkle_root_hash: [3; 32].into(),
			witness_merkle_root_hash: [4; 32].into(),
			time: 4,
			bits: 5.into(),
			nonce: 6,
		};

		let mut stream = Stream::default();
		stream.append(&block_header);

		let expected = vec![
			1, 0, 0, 0,
			2,
			0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
			1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
			3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
			4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
			4, 0, 0, 0,
			5, 0, 0, 0,
			6, 0, 0, 0,
		].into();

		assert_eq!(stream.out(), expected);
	}

	#[test]
	fn test_two_parent_blocks_header_reader() {
		let buffer = vec![
			1, 0, 0, 0,
			2, //
			0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
			1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
			3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3, 3,
			4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4, 4,
			4, 0, 0, 0,
			5, 0, 0, 0,
			6, 0, 0, 0,
		];

		let mut reader = Reader::new(&buffer);

		let previous_header_hash_1: H256 = [0; 32].into();
		let previous_header_hash_2: H256 = [1; 32].into();
		let hashes = vec![previous_header_hash_1, previous_header_hash_2];
		let expected = BlockHeader {
			version: 1,
			previous_header_hash: hashes,
			merkle_root_hash: [3; 32].into(),
			witness_merkle_root_hash: [4; 32].into(),
			time: 4,
			bits: 5.into(),
			nonce: 6,
		};

		assert_eq!(expected, reader.read().unwrap());
		assert_eq!(ReaderError::UnexpectedEnd, reader.read::<BlockHeader>().unwrap_err());
	}
}

