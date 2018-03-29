use chain::{Block, BlockHeader, Transaction, TransactionInput, TransactionOutput};
use chain::IndexedBlock;
use crypto::DHash256;
use std::sync::mpsc::Receiver;
use memory_pool::MemoryPoolRef;
use memory_pool::MemoryPoolOrderingStrategy as OrderingStrategy;
use memory_pool::MemoryPoolTransactionOutputProvider;
use message::types::{Block as BlockMessage, GetBlocks};
use message_wrapper::MessageWrapper;
use verification::BackwardsCompatibleChainVerifier as ChainVerifier;
use verification::{VerificationLevel, Verify};
use params::{ConsensusFork, ConsensusParams, NetworkParams};
use db::SharedStore;
use db::Error;

type BlockHeight = u32;

#[derive(Debug, PartialEq)]
pub enum Task {
    TryAcceptBlock(Block),
    TryAcceptTransaction(Transaction),
}

pub struct Acceptor {
    task_receiver: Receiver<Task>,
    message_wrapper: MessageWrapper,
    mempool: MemoryPoolRef,
    store: SharedStore,

    verifier: ChainVerifier,    
}

impl Acceptor {
    pub fn new(
        mempool: MemoryPoolRef,
        store: SharedStore,
        task_receiver: Receiver<Task>,
        message_wrapper: MessageWrapper,
        params: NetworkParams,
    ) -> Self {
        let verifier = ChainVerifier::new(
            store.clone(),
            ConsensusParams::new(NetworkParams::Mainnet, ConsensusFork::NoFork),
        );
        Acceptor {
            task_receiver,
            message_wrapper,
            mempool,
            store,
            verifier,
        }
    }

    pub fn run(&mut self) {
        loop {
            if let Ok(task) = self.task_receiver.recv() {
                match task {
                    Task::TryAcceptBlock(block) => self.try_accept_block(block),
                    Task::TryAcceptTransaction(transaction) => self.try_accept_transaction(transaction),
                }
            } else {
                break;
            }
        }
    }

    fn try_accept_block(&self, block: Block)
    {
        let block: IndexedBlock = block.into();
        match self.verifier.verify(VerificationLevel::Full, &block) {
            Ok(_) => self.add_verified_block(block),
            Err(err) => error!("Invalid block received: {:?}", err),
        }
    }


    fn add_and_canonize_block(&self, block: IndexedBlock) -> Result<(), Error> {
        let hash = block.hash().clone();
        match self.store.insert(block) {
            Ok(_) => self.store.canonize(&hash),
            Err(err) => Err(err),
        }
    }

    fn add_verified_block(&self, block: IndexedBlock) {
        let hash = block.hash().clone();
        let transactions = block.transactions.clone();
        match self.add_and_canonize_block(block) {
            Ok(_) => {
                info!("Block inserted and canonized with hash {}", hash);
                let mut mempool = self.mempool.write();
                for transaction in transactions {
                    mempool.remove_by_hash(&transaction.hash);
                }
            }
            Err(err) => error!("Cannot canonize received block due to {:?}", err),
        }
    }

    fn try_accept_transaction(&self, transaction: Transaction) {
        let hash = transaction.hash();
        if self.mempool.read().contains(&hash) {
            trace!(target: "handler", "Received transaction which already exists in mempool. Ignoring");
            return;
        }
        match MemoryPoolTransactionOutputProvider::for_transaction(
            self.store.clone(),
            &self.mempool,
            &transaction,
        ) {
            Err(e) => error!(
                "Can't accept transaction {} into mempool {:?}",
                transaction.hash(),
                e
            ),
            Ok(tx_output_provider) => {
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
                        // now insert transaction itself
                        memory_pool.insert_verified(transaction.into());
                    }
                    Err(e) => error!(
                        "Can't accept transaction {} into mempool {:?}",
                        transaction.hash(),
                        e
                    ),
                }
            }
        };
    }
}
