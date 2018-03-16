//! Bitcoin network
//! https://www.anintegratedworld.com/unravelling-the-mysterious-block-chain-magic-number/

use chain::Block;
use primitives::hash::H256;
use primitives::bigint::U256;

pub const MAGIC_MAINNET: u32 = 0x06A4D09A;
const MAGIC_TESTNET: u32 = 0x7E274A4D;

lazy_static! {
	static ref MAX_BITS_MAINNET: U256 = "00000000ffffffffffffffffffffffffffffffffffffffffffffffffffffffff".parse()
		.expect("hardcoded value should parse without errors");
	static ref MAX_BITS_TESTNET: U256 = "00000000ffffffffffffffffffffffffffffffffffffffffffffffffffffffff".parse()
		.expect("hardcoded value should parse without errors");
}

/// NetworkParams magic type.
pub type Magic = u32;

/// Bitcoin [network](https://bitcoin.org/en/glossary/mainnet)
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum NetworkParams {
	/// The original and main network for Bitcoin transactions, where satoshis have real economic value.
	Mainnet,
	/// The main bitcoin testnet.
	Testnet,
	/// Any other network. By default behaves like bitcoin mainnet.
	Other(u32),
}

impl NetworkParams {
	pub fn magic(&self) -> Magic {
		match *self {
			NetworkParams::Mainnet => MAGIC_MAINNET,
			NetworkParams::Testnet => MAGIC_TESTNET,
			NetworkParams::Other(value) => value,
		}
	}

	pub fn max_bits(&self) -> U256 {
		match *self {
			NetworkParams::Mainnet | NetworkParams::Other(_) => MAX_BITS_MAINNET.clone(),
			NetworkParams::Testnet => MAX_BITS_TESTNET.clone(),
		}
	}

	pub fn port(&self) -> u16 {
		match *self {
			NetworkParams::Mainnet | NetworkParams::Other(_)  => 6470,
			NetworkParams::Testnet => 16470,
		}
	}

	//Genesis block
	//public: 04c053e75152cd9ee02b0908864aeed81b5d100866c25482ae5ac4a1dfeaee8e99d2afc260dc81fda80e05767b5badd122932372f2330fd1ca1ca8874e39804683
	//address: 1KFoaRnZLw9DYhNVMfft84YHAVbLMRmWv5

	pub fn genesis_block(&self) -> Block {
		use chain::{Block, BlockHeader, Transaction, TransactionInput, TransactionOutput};
		match *self {
			NetworkParams::Mainnet | NetworkParams::Other(_) =>
			{
				let destination_locking_witness_program = "0014c83ef7b094d48e873f0e13db7892dfe5120418be".into();
				let transaction = Transaction {
					version: 0,
					inputs: vec![ TransactionInput::coinbase("0100".into()) ], //push 1 byte containing block height
					outputs: vec![ TransactionOutput {
						value: 50,
						script_pubkey: destination_locking_witness_program,
					}],
					lock_time: 0,
				};

				let mut block = Block
				{
					block_header: BlockHeader
					{
						version: 1,
						previous_header_hash: 0.into(),
						merkle_root_hash: 0.into(),
						witness_merkle_root_hash: 0.into(),
						time: 1234567,
						bits: 5.into(),
						nonce: 6,
					},
					transactions: vec![transaction]
				};

				block.block_header.merkle_root_hash = block.merkle_root();
				block.block_header.witness_merkle_root_hash = block.witness_merkle_root();

				block
			}
			NetworkParams::Testnet => "0100000000000000000000000000000000000000000000000000000000000000000000003ba3edfd7a7b12b27ac72c3e67768f617fc81bc3888a51323a9fb8aa4b1e5e4adae5494dffff001d1aa4ae180101000000010000000000000000000000000000000000000000000000000000000000000000ffffffff4d04ffff001d0104455468652054696d65732030332f4a616e2f32303039204368616e63656c6c6f72206f6e206272696e6b206f66207365636f6e64206261696c6f757420666f722062616e6b73ffffffff0100f2052a01000000434104678afdb0fe5548271967f1a67130b7105cd6a828e03909a67962e0ea1f61deb649f6bc3f4cef38c4f35504e51ec112de5c384df7ba0b8d578a4c702b6bf11d5fac00000000".into(),
		}
	}

	pub fn default_verification_edge(&self) -> H256 {
		 self.genesis_block().hash()
	}
}

#[cfg(test)]
mod tests {
	use compact::Compact;
	use super::{
		NetworkParams, MAGIC_MAINNET, MAGIC_TESTNET,
		MAX_BITS_MAINNET, MAX_BITS_TESTNET, MAX_BITS_REGTEST,
	};

	#[test]
	fn test_network_magic_number() {
		assert_eq!(MAGIC_MAINNET, NetworkParams::Mainnet.magic());
		assert_eq!(MAGIC_TESTNET, NetworkParams::Testnet.magic());
	}

	#[test]
	fn test_network_max_bits() {
		assert_eq!(NetworkParams::Mainnet.max_bits(), *MAX_BITS_MAINNET);
		assert_eq!(NetworkParams::Testnet.max_bits(), *MAX_BITS_TESTNET);
	}

	#[test]
	fn test_network_port() {
		assert_eq!(NetworkParams::Mainnet.port(), 6470);
		assert_eq!(NetworkParams::Testnet.port(), 16470);
	}
}
