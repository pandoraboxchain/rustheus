extern crate rustc_serialize;
extern crate heapsize;
extern crate primitives;
extern crate bitcrypto as crypto;
extern crate serialization as ser;
#[macro_use]
extern crate serialization_derive;
extern crate core;
extern crate keys;

pub mod constants;

mod block;
mod block_header;
mod merkle_root;

mod transaction;
mod transaction_helper;
mod payment_transaction;
mod penalty_transaction;

/// `IndexedBlock` extension
mod read_and_hash;
mod indexed_block;
mod indexed_header;
mod indexed_transaction;
mod commit_random_transaction;
mod reveal_random_transaction;
mod public_key_transaction;
mod private_key_transaction;
mod split_random_transaction;

pub trait RepresentH256 {
	fn h256(&self) -> hash::H256;
}

pub use rustc_serialize::hex;
pub use primitives::{hash, bytes, bigint, compact};

pub use block::Block;
pub use block_header::BlockHeader;
pub use merkle_root::{merkle_root, merkle_node_hash};
//pub use transaction::{Transaction};
pub use payment_transaction::{PaymentTransaction};
pub use transaction_helper::{TransactionInput, TransactionOutput, OutPoint};

pub use read_and_hash::{ReadAndHash, HashedData};
pub use indexed_block::IndexedBlock;
pub use indexed_header::IndexedBlockHeader;
pub use indexed_transaction::IndexedTransaction;

pub type ShortTransactionID = hash::H48;
