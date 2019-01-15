//! Bitcoin chain verifier

use hash::H256;
use chain::{IndexedBlock, IndexedBlockHeader, BlockHeader, PaymentTransaction};
use db::{SharedStore, TransactionOutputProvider, BlockHeaderProvider, BlockOrigin};
use params::ConsensusParams;
use error::{Error, TransactionError};
use canon::{CanonBlock, CanonTransaction};
use duplex_store::{DuplexTransactionOutputProvider, NoopStore};
use verify_chain::ChainVerifier;
use verify_header::HeaderVerifier;
use verify_transaction::MemoryPoolTransactionVerifier;
use accept_chain::ChainAcceptor;
use accept_transaction::MemoryPoolTransactionAcceptor;
use {Verify, VerificationLevel};

pub struct BackwardsCompatibleChainVerifier {
	store: SharedStore,
	consensus: ConsensusParams,
}

impl BackwardsCompatibleChainVerifier {
	pub fn new(store: SharedStore, consensus: ConsensusParams) -> Self {
		BackwardsCompatibleChainVerifier {
			store: store,
			consensus: consensus,
		}
	}

	fn verify_block(&self, verification_level: VerificationLevel, block: &IndexedBlock) -> Result<(), Error> {
		if verification_level == VerificationLevel::NoVerification {
			return Ok(());
		}

		let current_time = ::time::get_time().sec as u32;
		// first run pre-verification
		let chain_verifier = ChainVerifier::new(block, self.consensus.network, current_time);
		chain_verifier.check()?;

		assert_eq!(Some(self.store.best_block().hash), self.store.block_hash(self.store.best_block().number));
		let block_origin = self.store.block_origin(&block.header)?;
		trace!(target: "verification", "verify_block: {:?} best_block: {:?} block_origin: {:?}", block.hash(), self.store.best_block(), block_origin);
		match block_origin {
			BlockOrigin::KnownBlock => {
				warn!("Received block of known origin. It's worth to eliminate such cases for perfomance reasons")
			},
			BlockOrigin::CanonChain { block_number } => {
				let canon_block = CanonBlock::new(block);
				let chain_acceptor = ChainAcceptor::new(self.store.as_store(), &self.consensus, verification_level, canon_block, block_number);
				chain_acceptor.check()?;
			},
			BlockOrigin::SideChain(origin) => {
				let block_number = origin.block_number;
				let fork = self.store.fork(origin)?;
				let canon_block = CanonBlock::new(block);
				let chain_acceptor = ChainAcceptor::new(fork.store(), &self.consensus, verification_level, canon_block, block_number);
				chain_acceptor.check()?;
			},
			BlockOrigin::SideChainBecomingCanonChain(origin) => {
				let block_number = origin.block_number;
				let fork = self.store.fork(origin)?;
				let canon_block = CanonBlock::new(block);
				let chain_acceptor = ChainAcceptor::new(fork.store(), &self.consensus, verification_level, canon_block, block_number);
				chain_acceptor.check()?;
			},
		}

		assert_eq!(Some(self.store.best_block().hash), self.store.block_hash(self.store.best_block().number));
		Ok(())
	}

	pub fn verify_block_header(
		&self,
		_block_header_provider: &BlockHeaderProvider,
		hash: &H256,
		header: &BlockHeader
	) -> Result<(), Error> {
		// let's do only preverifcation
		// TODO: full verification
		let current_time = ::time::get_time().sec as u32;
		let header = IndexedBlockHeader::new(hash.clone(), header.clone());
		let header_verifier = HeaderVerifier::new(&header, self.consensus.network, current_time);
		header_verifier.check()
	}

	pub fn verify_mempool_transaction<T>(
		&self,
		prevout_provider: &T,
		height: u32,
		time: u32,
		transaction: &PaymentTransaction,
	) -> Result<(), TransactionError> where T: TransactionOutputProvider {
		let indexed_tx = transaction.clone().into();
		// let's do preverification first
		let tx_verifier = MemoryPoolTransactionVerifier::new(&indexed_tx, &self.consensus);
		try!(tx_verifier.check());

		let canon_tx = CanonTransaction::new(&indexed_tx);
		// now let's do full verification
		let noop = NoopStore;
		let output_store = DuplexTransactionOutputProvider::new(prevout_provider, &noop);
		let tx_acceptor = MemoryPoolTransactionAcceptor::new(
			self.store.as_transaction_meta_provider(),
			output_store,
			&self.consensus,
			canon_tx,
			height,
			time,
		);
		tx_acceptor.check()
	}
}

impl Verify for BackwardsCompatibleChainVerifier {
	fn verify(&self, level: VerificationLevel, block: &IndexedBlock) -> Result<(), Error> {
		let result = self.verify_block(level, block);
		trace!(
			target: "verification", "Block {} (transactions: {}) verification finished. Result {:?}",
			block.hash().to_reversed_str(),
			block.transactions.len(),
			result,
		);
		result
	}
}

#[cfg(test)]
mod tests {
	extern crate chain_builder;

	use std::sync::Arc;
	use chain::IndexedBlock;
	use db::{BlockChainDatabase, Error as DBError};
	use params::{NetworkParams, ConsensusParams, ConsensusFork};
	use script;
	use super::BackwardsCompatibleChainVerifier as ChainVerifier;
	use {Verify, Error, TransactionError, VerificationLevel};

	#[test]
	fn verify_orphan() {
		let storage = Arc::new(BlockChainDatabase::init_test_chain(vec![chain_builder::genesis().into()]));
		let b2 = chain_builder::block_h2().into();
		let verifier = ChainVerifier::new(storage, ConsensusParams::new(NetworkParams::Mainnet, ConsensusFork::NoFork));
		assert_eq!(Err(Error::Database(DBError::UnknownParent)), verifier.verify(VerificationLevel::Full, &b2));
	}

	#[test]
	fn verify_smoky() {
		let storage = Arc::new(BlockChainDatabase::init_test_chain(vec![chain_builder::genesis().into()]));
		let b1 = chain_builder::block_h1();
		let verifier = ChainVerifier::new(storage, ConsensusParams::new(NetworkParams::Mainnet, ConsensusFork::NoFork));
		assert!(verifier.verify(VerificationLevel::Full, &b1.into()).is_ok());
	}


	#[test]
	fn first_tx() {
		let storage = BlockChainDatabase::init_test_chain(
			vec![
				chain_builder::block_h0().into(),
				chain_builder::block_h1().into(),
			]);
		let b1 = chain_builder::block_h2();
		let verifier = ChainVerifier::new(Arc::new(storage), ConsensusParams::new(NetworkParams::Mainnet, ConsensusFork::NoFork));
		assert!(verifier.verify(VerificationLevel::Full, &b1.into()).is_ok());
	}

	#[test]
	fn coinbase_maturity() {
		let genesis = chain_builder::block_builder()
			.transaction()
				.coinbase()
				.output().value(50).build()
				.build()
			.merkled_header().build()
			.build();

		let storage = BlockChainDatabase::init_test_chain(vec![genesis.clone().into()]);
		let genesis_coinbase = genesis.transactions()[0].hash();

		let block = chain_builder::block_builder()
			.transaction()
				.coinbase()
				.output().value(1).build()
				.build()
			.transaction()
				.input().hash(genesis_coinbase).build()
				.output().value(2).build()
				.build()
			.merkled_header().parent(genesis.hash()).build()
			.build();

		let verifier = ChainVerifier::new(Arc::new(storage), ConsensusParams::new(NetworkParams::Mainnet, ConsensusFork::NoFork));

		let expected = Err(Error::Transaction(
			1,
			TransactionError::Maturity,
		));

		assert_eq!(expected, verifier.verify(VerificationLevel::Full, &block.into()));
	}

	#[test]
	fn non_coinbase_happy() {
		let genesis = chain_builder::block_builder()
			.transaction()
				.coinbase()
				.output().value(1).build()
				.build()
			.transaction()
				.output().value(50).build()
				.build()
			.merkled_header().build()
			.build();

		let storage = BlockChainDatabase::init_test_chain(vec![genesis.clone().into()]);
		let reference_tx = genesis.transactions()[1].hash();

		let block = chain_builder::block_builder()
			.transaction()
				.coinbase()
				.output().value(2).build()
				.build()
			.transaction()
				.input().hash(reference_tx).build()
				.output().value(1).build()
				.build()
			.merkled_header().parent(genesis.hash()).build()
			.build();

		let verifier = ChainVerifier::new(Arc::new(storage), ConsensusParams::new(NetworkParams::Mainnet, ConsensusFork::NoFork));
		assert!(verifier.verify(VerificationLevel::Full, &block.into()).is_ok());
	}

	#[test]
	fn transaction_references_same_block_happy() {
		let genesis = chain_builder::block_builder()
			.transaction()
				.coinbase()
				.output().value(1).build()
				.build()
			.transaction()
				.output().value(50).build()
				.build()
			.merkled_header().build()
			.build();

		let storage = BlockChainDatabase::init_test_chain(vec![genesis.clone().into()]);
		let first_tx_hash = genesis.transactions()[1].hash();

		let block = chain_builder::block_builder()
			.transaction()
				.coinbase()
				.output().value(2).build()
				.build()
			.transaction()
				.input().hash(first_tx_hash).build()
				.output().value(30).build()
				.output().value(20).build()
				.build()
			.derived_transaction(1, 0)
				.output().value(30).build()
				.build()
			.merkled_header().parent(genesis.hash()).build()
			.build();

		let verifier = ChainVerifier::new(Arc::new(storage), ConsensusParams::new(NetworkParams::Mainnet, ConsensusFork::NoFork));
		assert!(verifier.verify(VerificationLevel::Full, &block.into()).is_ok());
	}

	#[test]
	fn transaction_references_same_block_overspend() {
		let genesis = chain_builder::block_builder()
			.transaction()
				.coinbase()
				.output().value(1).build()
				.build()
			.transaction()
				.output().value(50).build()
				.build()
			.merkled_header().build()
			.build();

		let storage = BlockChainDatabase::init_test_chain(vec![genesis.clone().into()]);
		let first_tx_hash = genesis.transactions()[1].hash();

		let block = chain_builder::block_builder()
			.transaction()
				.coinbase()
				.output().value(2).build()
				.build()
			.transaction()
				.input().hash(first_tx_hash).build()
				.output().value(19).build()
				.output().value(31).build()
				.build()
			.derived_transaction(1, 0)
				.output().value(20).build()
				.build()
			.derived_transaction(1, 1)
				.output().value(20).build()
				.build()
			.merkled_header().parent(genesis.hash()).build()
			.build();

		let verifier = ChainVerifier::new(Arc::new(storage), ConsensusParams::new(NetworkParams::Mainnet, ConsensusFork::NoFork));

		let expected = Err(Error::Transaction(2, TransactionError::Overspend));
		assert_eq!(expected, verifier.verify(VerificationLevel::Full, &block.into()));
	}

	#[test]
	#[ignore]
	fn coinbase_happy() {
		let genesis = chain_builder::block_builder()
			.transaction()
				.coinbase()
				.output().value(50).build()
				.build()
			.merkled_header().build()
			.build();

		let storage = BlockChainDatabase::init_test_chain(vec![genesis.clone().into()]);
		let genesis_coinbase = genesis.transactions()[0].hash();

		// waiting 100 blocks for genesis coinbase to become valid
		for _ in 0..100 {
			let block: IndexedBlock = chain_builder::block_builder()
				.transaction().coinbase().build()
				.merkled_header().parent(genesis.hash()).build()
				.build()
				.into();
			let hash = block.hash().clone();
			storage.insert(block).expect("All dummy blocks should be inserted");
			storage.canonize(&hash).unwrap();
		}

		let best_hash = storage.best_block().hash;

		let block = chain_builder::block_builder()
			.transaction().coinbase().build()
			.transaction()
				.input().hash(genesis_coinbase.clone()).build()
				.build()
			.merkled_header().parent(best_hash).build()
			.build();

		let verifier = ChainVerifier::new(Arc::new(storage), ConsensusParams::new(NetworkParams::Mainnet, ConsensusFork::NoFork));
		assert!(verifier.verify(VerificationLevel::Full, &block.into()).is_ok());
	}

	#[test]
	fn absoulte_sigops_overflow_block() {
		let genesis = chain_builder::block_builder()
			.transaction()
				.coinbase()
				.build()
			.transaction()
				.output().value(50).build()
				.build()
			.merkled_header().build()
			.build();

		let storage = BlockChainDatabase::init_test_chain(vec![genesis.clone().into()]);
		let reference_tx = genesis.transactions()[1].hash();

		let mut builder_tx1 = script::Builder::default();
		for _ in 0..81000 {
			builder_tx1 = builder_tx1.push_opcode(script::Opcode::OP_CHECKSIG)
		}

		let mut builder_tx2 = script::Builder::default();
		for _ in 0..81001 {
			builder_tx2 = builder_tx2.push_opcode(script::Opcode::OP_CHECKSIG)
		}

		let block: IndexedBlock = chain_builder::block_builder()
			.transaction().coinbase().build()
			.transaction()
				.input()
					.hash(reference_tx.clone())
					.signature_bytes(builder_tx1.into_script().to_bytes())
					.build()
				.build()
			.transaction()
				.input()
					.hash(reference_tx)
					.signature_bytes(builder_tx2.into_script().to_bytes())
					.build()
				.build()
			.merkled_header().parent(genesis.hash()).build()
			.build()
			.into();

		let verifier = ChainVerifier::new(Arc::new(storage), ConsensusParams::new(NetworkParams::Mainnet, ConsensusFork::NoFork));
		let expected = Err(Error::MaximumSigops);
		assert_eq!(expected, verifier.verify(VerificationLevel::Full, &block.into()));
	}

	#[test]
	fn coinbase_overspend() {
		let genesis = chain_builder::block_builder()
			.transaction().coinbase().build()
			.merkled_header().build()
			.build();
		let storage = BlockChainDatabase::init_test_chain(vec![genesis.clone().into()]);

		let block: IndexedBlock = chain_builder::block_builder()
			.transaction()
				.coinbase()
				.output().value(5000000001).build()
				.build()
			.merkled_header().parent(genesis.hash()).build()
			.build()
			.into();

		let verifier = ChainVerifier::new(Arc::new(storage), ConsensusParams::new(NetworkParams::Mainnet, ConsensusFork::NoFork));

		let expected = Err(Error::CoinbaseOverspend {
			expected_max: 5000000000,
			actual: 5000000001
		});

		assert_eq!(expected, verifier.verify(VerificationLevel::Full, &block.into()));
	}
}
