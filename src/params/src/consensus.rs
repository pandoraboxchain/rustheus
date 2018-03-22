use {NetworkParams, Magic};

#[derive(Debug, Clone)]
/// Parameters that influence chain consensus.
pub struct ConsensusParams {
	/// Network.
	pub network: NetworkParams,
	/// Selected consensus fork.
	pub fork: ConsensusFork,
}

#[derive(Debug, Clone)]
/// Concurrent consensus rule forks.
pub enum ConsensusFork {
	/// No fork.
	NoFork
}

impl ConsensusParams {
	pub fn new(network: NetworkParams, fork: ConsensusFork) -> Self {
		ConsensusParams {
			network: network,
			fork: fork,
		}
	}

	pub fn magic(&self) -> Magic {
		self.network.magic()
	}
}

impl ConsensusFork {
	pub fn absolute_maximum_block_size() -> usize {
		8_000_000
	}

	/// Absolute (across all forks) maximum number of sigops in single block. Currently is max(sigops) for 8MB post-HF BitcoinCash block
	pub fn absolute_maximum_block_sigops() -> usize {
		160_000
	}

	pub fn max_transaction_size(&self) -> usize {
		// SegWit: size * 4 <= 4_000_000 ===> max size of tx is still 1_000_000
 		1_000_000
	}

	pub fn min_block_size(&self) -> usize {
		0
	}

	pub fn max_block_size(&self) -> usize {
		4_000_000
	}

	pub fn max_block_sigops(&self, _height: u32, _block_size: usize) -> usize {
		80_000
	}

	pub fn max_block_sigops_cost(&self, _height: u32, _block_size: usize) -> usize {
		80_000
	}
}

#[cfg(test)]
mod tests {
	use super::{ConsensusParams, ConsensusFork};
	//TODO do we need miner_confirmation_window and rule_change_activation_threshold ?
	#[test]
	fn test_consensus_fork_min_block_size() {
		assert_eq!(ConsensusFork::NoFork.min_block_size(), 0);
	}

	#[test]
	fn test_consensus_fork_max_transaction_size() {
		assert_eq!(ConsensusFork::NoFork.max_transaction_size(), 1_000_000);
	}

	#[test]
	fn test_consensus_fork_max_block_sigops() {
		assert_eq!(ConsensusFork::NoFork.max_block_sigops(0, 1_000_000), 80_000);
	}
}
