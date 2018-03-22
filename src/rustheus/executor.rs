use chain::{Block, BlockHeader, Transaction, TransactionInput, TransactionOutput};
use chain::IndexedBlock;
use crypto::DHash256;
use std::sync::mpsc::Receiver;
use memory_pool::MemoryPoolRef;
use memory_pool::MemoryPoolOrderingStrategy as OrderingStrategy;
use std::time::{SystemTime, UNIX_EPOCH};
use message::types::{Block as BlockMessage, GetBlocks};
use message_wrapper::MessageWrapper;
use db::SharedStore;
use keys::Address;
use script::Builder;
use primitives::hash::H256;
use db::Error;

type BlockHeight = u32;

#[derive(Debug, PartialEq)]
pub enum Task {
    AddVerifiedBlock(IndexedBlock),
    SignBlock(Address),
    RequestLatestBlocks(),

    //debug and explore
    GetTransactionMeta(H256),
    GetBlockHash(u32),
}

pub struct Executor {
    task_receiver: Receiver<Task>,
    message_wrapper: MessageWrapper,
    mempool: MemoryPoolRef,
    store: SharedStore,
}

impl Executor {
    pub fn new(
        mempool: MemoryPoolRef,
        store: SharedStore,
        task_receiver: Receiver<Task>,
        message_wrapper: MessageWrapper,
    ) -> Self {
        Executor {
            task_receiver,
            message_wrapper,
            mempool,
            store,
        }
    }

    pub fn run(&mut self) {
        loop {
            if let Ok(task) = self.task_receiver.recv() {
                info!("task received, it is {:?}", task);
                match task {
                    Task::SignBlock(coinbase_recipient) => self.sign_block(coinbase_recipient),
                    Task::GetTransactionMeta(hash) => self.get_transaction_meta(hash),
                    Task::GetBlockHash(height) => self.get_block_hash(height),
                    Task::RequestLatestBlocks() => self.request_latest_blocks(),
                    Task::AddVerifiedBlock(block) => self.add_verified_block(block),
                }
            } else {
                break;
            }
        }
    }

    fn sign_block(&mut self, coinbase_recipient: Address) {
        let current_time = SystemTime::now();
        let time_since_the_epoch = current_time
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        let header = BlockHeader {
            version: 1,
            previous_header_hash: self.store.best_block().hash,
            merkle_root_hash: DHash256::default().finish(),
            witness_merkle_root_hash: Default::default(),
            time: time_since_the_epoch.as_secs() as u32,
            bits: 5.into(),
            nonce: 6,
        };
        let mut mempool = self.mempool.write();
        let mut transactions = vec![self.create_coinbase(coinbase_recipient)];
        //TODO add transaction fees to coinbase reward
        //TODO take not fixed number of transactions, but deduce it from block size
        let indexed_transactions =
            mempool.remove_n_with_strategy(50, OrderingStrategy::ByTransactionScore);
        let block_tx: Vec<Transaction> =
            indexed_transactions.into_iter().map(|tx| tx.raw).collect();
        transactions.extend(block_tx);
        let mut block = Block::new(header, transactions);

        //recalculate merkle root
        block.block_header.merkle_root_hash = block.merkle_root();
        block.block_header.witness_merkle_root_hash = block.witness_merkle_root();

        self.add_and_canonize_block(block.clone().into())
            .expect("Error inserting block");

        let block_message = BlockMessage { block };
        self.message_wrapper.broadcast(&block_message);
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

    fn create_coinbase(&self, recipient: Address) -> Transaction {
        let block_height = self.store.best_block().number + 1;

        //add block height as coinbase prefix
        let prefix = Builder::default()
            .push_num(block_height.into())
            .into_script();

        Transaction {
            version: 0,
            inputs: vec![TransactionInput::coinbase(prefix.into())],
            outputs: vec![
                TransactionOutput {
                    value: 50,
                    script_pubkey: Builder::build_p2pkh(&recipient.hash).to_bytes(),
                },
            ],
            lock_time: 0,
        }
    }

    fn get_transaction_meta(&self, hash: H256) {
        match self.store.transaction_meta(&hash) {
            Some(meta) => debug!("Meta is {:?}", meta),
            None => error!("No transaction with such hash"),
        }
    }

    fn get_block_hash(&self, height: u32) {
        match self.store.block_hash(height) {
            Some(hash) => debug!("Block hash is {:?}", hash),
            None => error!("No block at this height"),
        }
    }

    fn request_latest_blocks(&self) {
        info!("Requesting latest blocks from network");
        let index = self.store.best_block().number;
        let step = 1u32;
        let block_locator_hashes = self.block_locator_hashes_for_storage(index, step);
        let get_blocks_msg = GetBlocks::with_block_locator_hashes(block_locator_hashes);
        self.message_wrapper.broadcast(&get_blocks_msg);
    }

    /// Calculate block locator hashes for store
    fn block_locator_hashes_for_storage(
        &self,
        mut index: BlockHeight,
        mut step: BlockHeight,
    ) -> Vec<H256> {
        let mut hashes = vec![];

        loop {
            let block_hash = self.store
                .block_hash(index)
                .expect("private function; index calculated in `block_locator_hashes`; qed");
            hashes.push(block_hash);

            if hashes.len() >= 10 {
                step <<= 1;
            }
            if index < step {
                // always include genesis hash
                if index != 0 {
                    let genesis_block_hash = self.store
                        .block_hash(0)
                        .expect("No genesis block found at height 0");
                    hashes.push(genesis_block_hash);
                }

                break;
            }
            index -= step;
        }

        hashes
    }
}
