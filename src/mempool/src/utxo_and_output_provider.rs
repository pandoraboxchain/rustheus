use chain::{Transaction, TransactionOutput, OutPoint};
use db::{TransactionOutputProvider, TransactionUtxoProvider, SharedStore};
use memory_pool::{DoubleSpendCheckResult, HashedOutPoint, NonFinalDoubleSpendSet, MemoryPoolRef};
use verification::TransactionError;
use primitives::hash::H160;

type StorageRef = SharedStore;

pub struct UtxoAndOutputProvider {
	storage: StorageRef,
	mempool: MemoryPoolRef,
}

impl UtxoAndOutputProvider {
	/// TODO can we take only read reference to mempool?
	pub fn new(storage: StorageRef, mempool: MemoryPoolRef) -> Self {
		UtxoAndOutputProvider {
			storage,
			mempool,
		}
	}
}

impl TransactionUtxoProvider for UtxoAndOutputProvider {
	fn transaction_with_output_address(&self, address: &H160) -> Vec<OutPoint> {
		self.storage
            .transaction_with_output_address(&address)
            .into_iter()
            .chain(self.mempool.read().transaction_with_output_address(&address).into_iter())
            .filter(|outpoint| !self.mempool.read().is_spent(outpoint))			
			.collect()
	}
}
//Copy from DuplexTransactionOutputProvider
impl TransactionOutputProvider for UtxoAndOutputProvider {
	fn transaction_output(&self, prevout: &OutPoint, transaction_index: usize) -> Option<TransactionOutput> {
		self.mempool.read().transaction_output(prevout, transaction_index)
			.or_else(|| self.storage.transaction_output(prevout, transaction_index))
	}

	fn is_spent(&self, prevout: &OutPoint) -> bool {
		self.mempool.read().is_spent(prevout) || self.storage.is_spent(prevout)
	}
}