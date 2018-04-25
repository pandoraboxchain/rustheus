use chain::IndexedBlock;
use chain::{Block, Transaction};
use db::Error as DBError;
use db::SharedStore;
use memory_pool::MemoryPoolRef;
use memory_pool::MemoryPoolTransactionOutputProvider;
use params::{ConsensusFork, ConsensusParams, NetworkParams};
use primitives::hash::H256;
use verification::BackwardsCompatibleChainVerifier as ChainVerifier;
use verification::{Error, TransactionError};
use verification::{VerificationLevel, Verify};

use futures::done;
use futures::prelude::*;
use futures_cpupool::CpuPool;
use std::sync::Arc;

pub type AcceptorRef = Arc<Acceptor>;

pub struct Acceptor {
    //message_wrapper: MessageWrapper,
    mempool: MemoryPoolRef,
    store: SharedStore,
    cpupool: CpuPool,

    verifier: ChainVerifier,
}

impl Acceptor {
    pub fn new(
        mempool: MemoryPoolRef,
        store: SharedStore,
        //message_wrapper: MessageWrapper,
        params: NetworkParams,
        cpupool: CpuPool,
    ) -> Self {
        let verifier = ChainVerifier::new(
            store.clone(),
            ConsensusParams::new(NetworkParams::Mainnet, ConsensusFork::NoFork),
        );
        Acceptor {
            //message_wrapper,
            mempool,
            store,
            verifier,
            cpupool,
        }
    }

    pub fn accept_transaction(
        &self,
        transaction: Transaction,
    ) -> impl Future<Item = Transaction, Error = TransactionError> {
        let future = self.async_accept_transaction(transaction);
        self.cpupool.spawn(future)
    }

    pub fn accept_block(&self, block: Block) -> impl Future<Item = H256, Error = Error> {
        let future = self.async_accept_block(block);
        self.cpupool.spawn(future)
    }

    pub fn async_accept_transaction(
        &self,
        transaction: Transaction,
    ) -> impl Future<Item = Transaction, Error = TransactionError> {
        done(self.try_accept_transaction(transaction))
    }

    pub fn async_accept_block(&self, block: Block) -> impl Future<Item = H256, Error = Error> {
        done(self.try_accept_block(block))
    }

    fn try_accept_block(&self, block: Block) -> Result<H256, Error> {
        let block: IndexedBlock = block.into();
        match self.verifier.verify(VerificationLevel::Full, &block) {
            Ok(_) => self.add_verified_block(block).map_err(Error::Database),
            Err(err) => {
                error!("Invalid block received: {:?}", err);
                return Err(err);
            }
        }
    }

    fn add_and_canonize_block(&self, block: IndexedBlock) -> Result<(), DBError> {
        let hash = block.hash().clone();
        try!(self.store.insert(block));
        try!(self.store.canonize(&hash));
        Ok(())
    }

    fn add_verified_block(&self, block: IndexedBlock) -> Result<H256, DBError> {
        let hash = block.hash().clone();
        let transactions = block.transactions.clone();
        match self.add_and_canonize_block(block) {
            Ok(_) => {
                info!("Block inserted and canonized with hash {}", hash);
                let mut mempool = self.mempool.write();
                for transaction in transactions {
                    mempool.remove_by_hash(&transaction.hash);
                }
                return Ok(hash);
            }
            Err(err) => {
                error!("Cannot canonize received block due to {:?}", err);
                return Err(err);
            }
        }
    }

    fn try_accept_transaction(&self, transaction: Transaction) -> Result<Transaction, TransactionError> {
        let hash = transaction.hash();
        if self.mempool.read().contains(&hash) {
            trace!(target: "handler", "Received transaction which already exists in mempool. Ignoring");
            return Ok(transaction);
        }
        match MemoryPoolTransactionOutputProvider::for_transaction(
            self.store.clone(),
            &self.mempool,
            &transaction,
        ) {
            Ok(tx_output_provider) => {
                self.try_accept_transaction_with_output_provider(transaction, tx_output_provider)
            }
            Err(e) => {
                error!(
                    "Can't accept transaction {} into mempool {:?}",
                    transaction.hash(),
                    e
                );
                return Err(e);
            }
        }
    }

    fn try_accept_transaction_with_output_provider(
        &self,
        transaction: Transaction,
        tx_output_provider: MemoryPoolTransactionOutputProvider,
    ) -> Result<Transaction, TransactionError> {
        let height = self.store.best_block().number;
        match self.verifier.verify_mempool_transaction(
            &tx_output_provider,
            height,
            /*time*/ 0,
            &transaction,
        ) {
            Ok(_) => {
                // we have verified transaction, but possibly this transaction replaces
                // existing transaction from memory pool
                // => remove previous transactions before
                let mut memory_pool = self.mempool.write();
                for input in &transaction.inputs {
                    memory_pool.remove_by_prevout(&input.previous_output);
                }
                let transaction_clone = transaction.clone();
                // now insert transaction itself
                memory_pool.insert_verified(transaction.into());
                return Ok(transaction_clone);
            }
            Err(e) => {
                error!(
                    "Can't accept transaction {} into mempool {:?}",
                    transaction.hash(),
                    e
                );
                return Err(e.into());
            }
        }
    }
}
