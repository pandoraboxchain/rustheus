use chain::{Block, BlockHeader, Transaction, TransactionInput,
            TransactionOutput};
use crypto::DHash256;
use std::sync::mpsc::{self, Receiver, Sender};
use memory_pool::MemoryPoolRef;
use memory_pool::MemoryPoolOrderingStrategy as OrderingStrategy;
use std::time::{SystemTime, UNIX_EPOCH};
use message::types::{Block as BlockMessage, GetBlocks};
use message_wrapper::MessageWrapper;
use db::SharedStore;
use keys::Address;
use script::Builder;
use primitives::hash::H256;

type BlockHeight = u32;

#[derive(Debug, PartialEq)]
pub enum ExecutorTask {
    SignBlock(Address),
    RequestLatestBlocks(),

    //debug and explore
    GetTransactionMeta(H256),
    GetBlockHash(u32),
}

pub struct Executor {
    task_receiver: Receiver<ExecutorTask>,
    message_wrapper: MessageWrapper,
    mempool: MemoryPoolRef,
    storage: SharedStore,
}

impl Executor {
    pub fn new(
        mempool: MemoryPoolRef,
        storage: SharedStore,
        message_wrapper: MessageWrapper,
    ) -> (Self, Sender<ExecutorTask>) {
        let (task_sender, task_receiver) = mpsc::channel();
        let executor = Executor {
            task_receiver,
            message_wrapper,
            mempool,
            storage,
        };
        (executor, task_sender)
    }

    pub fn run(&mut self) {
        loop {
            if let Ok(task) = self.task_receiver.recv() {
                info!("task received, it is {:?}", task);
                match task {
                    ExecutorTask::SignBlock(coinbase_recipient) => {
                        self.sign_block(coinbase_recipient)
                    }
                    ExecutorTask::GetTransactionMeta(hash) => self.get_transaction_meta(hash),
                    ExecutorTask::GetBlockHash(height) => self.get_block_hash(height),
                    ExecutorTask::RequestLatestBlocks() => self.request_latest_blocks(),
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
            previous_header_hash: self.storage.best_block().hash,
            merkle_root_hash: DHash256::default().finish(),
            time: time_since_the_epoch.as_secs() as u32,
            bits: 5.into(),
            nonce: 6,
        };
        let mut mempool = self.mempool.write().unwrap();
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

        let block_message = BlockMessage { block };
        self.message_wrapper.broadcast(&block_message);
    }

    fn create_coinbase(&self, recipient: Address) -> Transaction {
        use chain::bytes::Bytes;
        Transaction {
            version: 0,
            inputs: vec![TransactionInput::coinbase(Bytes::default())],
            outputs: vec![
                TransactionOutput {
                    value: 50,
                    script_pubkey: Builder::build_p2pkh(&recipient.hash).to_bytes(),
                },
            ],
            lock_time: self.storage.best_block().number + 1, //use lock_time as uniqueness provider for coinbase transaction
        }
    }

    fn get_transaction_meta(&self, hash: H256) {
        match self.storage.transaction_meta(&hash) {
            Some(meta) => debug!("Meta is {:?}", meta),
            None => error!("No transaction with such hash"),
        }
    }

    fn get_block_hash(&self, height: u32) {
        match self.storage.block_hash(height) {
            Some(hash) => debug!("Block hash is {:?}", hash),
            None => error!("No block at this height"),
        }
    }

    fn request_latest_blocks(&self) {
        info!("Requesting latest blocks from network");
        let index = self.storage.best_block().number;
        let step = 1u32;
        let block_locator_hashes = self.block_locator_hashes_for_storage(index, step);
        let get_blocks_msg = GetBlocks::with_block_locator_hashes(block_locator_hashes);
        self.message_wrapper.broadcast(&get_blocks_msg);
    }

    /// Calculate block locator hashes for storage
    fn block_locator_hashes_for_storage(
        &self,
        mut index: BlockHeight,
        mut step: BlockHeight,
    ) -> Vec<H256> {
        let mut hashes = vec![];

        loop {
            let block_hash = self.storage
                .block_hash(index)
                .expect("private function; index calculated in `block_locator_hashes`; qed");
            hashes.push(block_hash);

            if hashes.len() >= 10 {
                step <<= 1;
            }
            if index < step {
                // always include genesis hash
                if index != 0 {
                    let genesis_block_hash = self.storage
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
