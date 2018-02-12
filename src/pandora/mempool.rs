use chain::Transaction;
use std::sync::{Arc, RwLock};

pub type MempoolRef = Arc<RwLock<Mempool>>;

/// Wrapper around `Vec<Transaction>`
pub struct Mempool
{
    pub transactions: Vec<Transaction>
}

impl Mempool
{
    pub fn new() -> Self
    {
        let mempool = Mempool { transactions: Vec::new() };
        mempool
    }
}