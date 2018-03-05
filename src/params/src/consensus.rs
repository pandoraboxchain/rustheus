use {NetworkParams, Magic, Deployment};

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

	/// Witness scale factor (equal among all forks)
	pub fn witness_scale_factor() -> usize {
		4
	}

	pub fn activation_height(&self) -> u32 {
		0
	}

	pub fn max_transaction_size(&self) -> usize {
		// SegWit: size * 4 <= 4_000_000 ===> max size of tx is still 1_000_000
 		1_000_000
	}

	pub fn min_block_size(&self, height: u32) -> usize {
		0
	}

	pub fn max_block_size(&self, height: u32) -> usize {
		1_000_000
	}

	pub fn max_block_sigops(&self, height: u32, block_size: usize) -> usize {
		20_000
	}

	pub fn max_block_sigops_cost(&self, height: u32, block_size: usize) -> usize {
		80_000
	}

	pub fn max_block_weight(&self, _height: u32) -> usize {
		4_000_000
	}
}

#[cfg(test)]
mod tests {
	use super::super::Network;
	use super::{ConsensusParams, ConsensusFork};

	#[test]
	fn test_consensus_params_bip34_height() {
		assert_eq!(ConsensusParams::new(Network::Mainnet, ConsensusFork::NoFork).bip34_height, 227931);
		assert_eq!(ConsensusParams::new(Network::Testnet, ConsensusFork::NoFork).bip34_height, 21111);
		assert_eq!(ConsensusParams::new(Network::Regtest, ConsensusFork::NoFork).bip34_height, 100000000);
	}

	#[test]
	fn test_consensus_params_bip65_height() {
		assert_eq!(ConsensusParams::new(Network::Mainnet, ConsensusFork::NoFork).bip65_height, 388381);
		assert_eq!(ConsensusParams::new(Network::Testnet, ConsensusFork::NoFork).bip65_height, 581885);
		assert_eq!(ConsensusParams::new(Network::Regtest, ConsensusFork::NoFork).bip65_height, 1351);
	}

	#[test]
	fn test_consensus_params_bip66_height() {
		assert_eq!(ConsensusParams::new(Network::Mainnet, ConsensusFork::NoFork).bip66_height, 363725);
		assert_eq!(ConsensusParams::new(Network::Testnet, ConsensusFork::NoFork).bip66_height, 330776);
		assert_eq!(ConsensusParams::new(Network::Regtest, ConsensusFork::NoFork).bip66_height, 1251);
	}

	#[test]
	fn test_consensus_activation_threshold() {
		assert_eq!(ConsensusParams::new(Network::Mainnet, ConsensusFork::NoFork).rule_change_activation_threshold, 1916);
		assert_eq!(ConsensusParams::new(Network::Testnet, ConsensusFork::NoFork).rule_change_activation_threshold, 1512);
		assert_eq!(ConsensusParams::new(Network::Regtest, ConsensusFork::NoFork).rule_change_activation_threshold, 108);
	}

	#[test]
	fn test_consensus_miner_confirmation_window() {
		assert_eq!(ConsensusParams::new(Network::Mainnet, ConsensusFork::NoFork).miner_confirmation_window, 2016);
		assert_eq!(ConsensusParams::new(Network::Testnet, ConsensusFork::NoFork).miner_confirmation_window, 2016);
		assert_eq!(ConsensusParams::new(Network::Regtest, ConsensusFork::NoFork).miner_confirmation_window, 144);
	}

	#[test]
	fn test_consensus_fork_min_block_size() {
		assert_eq!(ConsensusFork::NoFork.min_block_size(0), 0);
	}

	#[test]
	fn test_consensus_fork_max_transaction_size() {
		assert_eq!(ConsensusFork::NoFork.max_transaction_size(), 1_000_000);
	}

	#[test]
	fn test_consensus_fork_max_block_sigops() {
		assert_eq!(ConsensusFork::NoFork.max_block_sigops(0, 1_000_000), 20_000);
	}
}
