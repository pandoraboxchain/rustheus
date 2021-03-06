use params::{ConsensusParams};
use db::{TransactionOutputProvider, BlockHeaderProvider};
use script;
use sigops::{transaction_sigops, transaction_sigops_cost}	;
use work::block_reward_satoshi;
use duplex_store::DuplexTransactionOutputProvider;
use canon::CanonBlock;
use error::{Error, TransactionError};
use timestamp::median_timestamp;

/// Flexible verification of ordered block
pub struct BlockAcceptor<'a> {
	pub finality: BlockFinality<'a>,
	pub serialized_size: BlockSerializedSize<'a>,
	pub sigops: BlockSigops<'a>,
	pub coinbase_claim: BlockCoinbaseClaim<'a>,
	pub coinbase_script: BlockCoinbaseScript<'a>,
	pub witness: BlockWitness<'a>,
}

impl<'a> BlockAcceptor<'a> {
	pub fn new(
		store: &'a TransactionOutputProvider,
		consensus: &'a ConsensusParams,
		block: CanonBlock<'a>,
		height: u32,
		headers: &'a BlockHeaderProvider,
	) -> Self {
		BlockAcceptor {
			finality: BlockFinality::new(block, height, headers),
			serialized_size: BlockSerializedSize::new(block, consensus),
			coinbase_script: BlockCoinbaseScript::new(block, height),
			coinbase_claim: BlockCoinbaseClaim::new(block, store, height),
			sigops: BlockSigops::new(block, store, consensus, height),
			witness: BlockWitness::new(block),
		}
	}

	pub fn check(&self) -> Result<(), Error> {
		self.finality.check()?;
		self.sigops.check()?;
		self.serialized_size.check()?;
		self.coinbase_claim.check()?;
		self.coinbase_script.check()?;
		self.witness.check()?;
		Ok(())
	}
}

pub struct BlockFinality<'a> {
	block: CanonBlock<'a>,
	height: u32,
	csv_active: bool,
	headers: &'a BlockHeaderProvider,
}

impl<'a> BlockFinality<'a> {
	fn new(block: CanonBlock<'a>, height: u32, headers: &'a BlockHeaderProvider) -> Self {

		BlockFinality {
			block: block,
			height: height,
			csv_active: true,
			headers: headers,
		}
	}

	fn check(&self) -> Result<(), Error> {
		let time_cutoff = if self.csv_active {
			median_timestamp(&self.block.header.raw, self.headers)
		} else {
			self.block.header.raw.time
		};

		if self.block.transactions.iter().all(|tx| tx.raw.is_final_in_block(self.height, time_cutoff)) {
			Ok(())
		} else {
			Err(Error::NonFinalBlock)
		}
	}
}

pub struct BlockSerializedSize<'a> {
	block: CanonBlock<'a>,
	consensus: &'a ConsensusParams,
}

impl<'a> BlockSerializedSize<'a> {
	fn new(block: CanonBlock<'a>, consensus: &'a ConsensusParams) -> Self {
		BlockSerializedSize {
			block: block,
			consensus: consensus,
		}
	}

	fn check(&self) -> Result<(), Error> {
		let size = self.block.size();

		if size < self.consensus.fork.min_block_size() ||
			size > self.consensus.fork.max_block_size() {
			return Err(Error::Size(size));
		}

		Ok(())
	}
}

pub struct BlockSigops<'a> {
	block: CanonBlock<'a>,
	store: &'a TransactionOutputProvider,
	consensus: &'a ConsensusParams,
	height: u32,
}

impl<'a> BlockSigops<'a> {
	fn new(block: CanonBlock<'a>, store: &'a TransactionOutputProvider, consensus: &'a ConsensusParams, height: u32) -> Self {
		BlockSigops {
			block: block,
			store: store,
			consensus: consensus,
			height: height,
		}
	}

	fn check(&self) -> Result<(), Error> {
		let store = DuplexTransactionOutputProvider::new(self.store, &*self.block);
		let (sigops, sigops_cost) = self.block.transactions.iter()
			.map(|tx| {
				let tx_sigops = transaction_sigops(&tx.raw, &store);
				let tx_sigops_cost = transaction_sigops_cost(&tx.raw, &store, tx_sigops);
				(tx_sigops, tx_sigops_cost)
			})
			.fold((0, 0), |acc, (tx_sigops, tx_sigops_cost)| (acc.0 + tx_sigops, acc.1 + tx_sigops_cost));

		// sigops check is valid for all forks:
		// before SegWit: 20_000
		// after SegWit: cost of sigops is sigops * 4 and max cost is 80_000 => max sigops is still 20_000
		let size = self.block.size();
		if sigops > self.consensus.fork.max_block_sigops(self.height, size) {
			return Err(Error::MaximumSigops);
		}

		// sigops check is valid for all forks:
		// before SegWit: no witnesses => cost is sigops * 4 and max cost is 80_000
		// after SegWit: it is main check for sigops
		if sigops_cost > self.consensus.fork.max_block_sigops_cost(self.height, size) {
			Err(Error::MaximumSigopsCost)
		} else {
			Ok(())
		}
	}
}

pub struct BlockCoinbaseClaim<'a> {
	block: CanonBlock<'a>,
	store: &'a TransactionOutputProvider,
	height: u32,
}

impl<'a> BlockCoinbaseClaim<'a> {
	fn new(block: CanonBlock<'a>, store: &'a TransactionOutputProvider, height: u32) -> Self {
		BlockCoinbaseClaim {
			block: block,
			store: store,
			height: height,
		}
	}

	fn check(&self) -> Result<(), Error> {
		let store = DuplexTransactionOutputProvider::new(self.store, &*self.block);

		let mut fees: u64 = 0;

		for (tx_idx, tx) in self.block.transactions.iter().enumerate().skip(1) {
			// (1) Total sum of all referenced outputs
			let mut incoming: u64 = 0;
			for input in tx.raw.inputs.iter() {
				let (sum, overflow) = incoming.overflowing_add(
					store.transaction_output(&input.previous_output, tx_idx).map(|o| o.value).unwrap_or(0));
				if overflow {
					return Err(Error::ReferencedInputsSumOverflow);
				}
				incoming = sum;
			}

			// (2) Total sum of all outputs
			let spends = tx.raw.total_spends();

			// Difference between (1) and (2)
			let (difference, overflow) = incoming.overflowing_sub(spends);
			if overflow {
				return Err(Error::Transaction(tx_idx, TransactionError::Overspend))
			}

			// Adding to total fees (with possible overflow)
			let (sum, overflow) = fees.overflowing_add(difference);
			if overflow {
				return Err(Error::TransactionFeesOverflow)
			}

			fees = sum;
		}

		let claim = self.block.transactions[0].raw.total_spends();

		let (reward, overflow) = fees.overflowing_add(block_reward_satoshi(self.height));
		if overflow {
			return Err(Error::TransactionFeeAndRewardOverflow);
		}

		if claim > reward {
			Err(Error::CoinbaseOverspend { expected_max: reward, actual: claim })
		} else {
			Ok(())
		}
	}
}

pub struct BlockCoinbaseScript<'a> {
	block: CanonBlock<'a>,
	height: u32,
}

impl<'a> BlockCoinbaseScript<'a> {
	fn new(block: CanonBlock<'a>, height: u32) -> Self {
		BlockCoinbaseScript {
			block: block,
			height: height,
		}
	}

	fn check(&self) -> Result<(), Error> {
		let prefix = script::Builder::default()
			.push_num(self.height.into())
			.into_script();

		let matches = self.block.transactions.first()
			.and_then(|tx| tx.raw.inputs.first())
			.map(|input| input.script_sig.starts_with(&prefix))
			.unwrap_or(false);

		if matches {
			Ok(())
		} else {
			Err(Error::CoinbaseScript)
		}
	}
}

pub struct BlockWitness<'a> {
	block: CanonBlock<'a>,
}

impl<'a> BlockWitness<'a> {
	fn new(block: CanonBlock<'a>) -> Self {
		BlockWitness {
			block: block,
		}
	}

	fn check(&self) -> Result<(), Error> {
		let witness_from_header = &self.block.header().raw.witness_merkle_root_hash;
		let witness_calculated = self.block.raw().witness_merkle_root();

		if witness_calculated != *witness_from_header {
			return Err(Error::WitnessMerkleCommitmentMismatch);
		}

		Ok(())
	}
}

#[cfg(test)]
mod tests {
	extern crate chain_builder;

	use {Error, CanonBlock};
	use super::BlockCoinbaseScript;

	#[test]
	fn test_block_coinbase_script() {
		// transaction from block 461373
		// https://blockchain.info/rawtx/7cf05175ce9c8dbfff9aafa8263edc613fc08f876e476553009afcf7e3868a0c?format=hex
		let tx = "01000000010000000000000000000000000000000000000000000000000000000000000000ffffffff3f033d0a070004b663ec58049cba630608733867a0787a02000a425720537570706f727420384d200a666973686572206a696e78696e092f425720506f6f6c2fffffffff01903d9d4e000000001976a914721afdf638d570285d02d3076d8be6a03ee0794d88ac00000000".into();
		let block_number = 461373;
		let block = chain_builder::block_builder()
			.with_transaction(tx)
			.header().build()
			.build()
			.into();

		let coinbase_script_validator = BlockCoinbaseScript {
			block: CanonBlock::new(&block),
			height: block_number,
		};

		assert_eq!(coinbase_script_validator.check(), Ok(()));

		let coinbase_script_validator2 = BlockCoinbaseScript {
			block: CanonBlock::new(&block),
			height: block_number - 1,
		};

		assert_eq!(coinbase_script_validator2.check(), Err(Error::CoinbaseScript));
	}
}
