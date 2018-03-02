use chain::Transaction;
use std::sync::{Arc, RwLock};
use std::collections::HashMap;
use primitives::hash::H256;

pub type MempoolRef = Arc<RwLock<Mempool>>;

/// Wrapper around `Vec<Transaction>`
pub struct Mempool {
    pub transactions: HashMap<H256, Transaction>,
}

impl Mempool {
    pub fn new() -> Self {
        let mempool = Mempool {
            transactions: HashMap::new(),
        };
        mempool
    }

    pub fn insert(&mut self, transaction: Transaction) {
        let hash = transaction.hash();
        self.transactions.insert(hash, transaction);
    }

    //Clear mempool returning transactions as Vector
    pub fn drain_as_vec(&mut self) -> Vec<Transaction> {
        self.transactions
            .drain()
            .map(|(_, transaction)| transaction.clone())
            .collect()
    }

    pub fn remove_transactions(&mut self, transactions: Vec<Transaction>) {
        for transaction in transactions {
            self.transactions.remove(&transaction.hash());
        }
    }
}
