use chain::{BlockHeader, Block, Transaction, TransactionInput, TransactionOutput, OutPoint};
use crypto::DHash256;
use std::sync::mpsc::{self, Sender, Receiver};
use mempool::{MempoolRef};
use std::time::{SystemTime, UNIX_EPOCH};
use executor_tasks::Task;
use message::types::{Tx, Block as BlockMessage};
use message_wrapper::MessageWrapper;
use db::SharedStore;

pub struct Executor
{
    task_receiver: Receiver<Task>,
    task_sender: Sender<Task>,
    //mempool_sender: Sender<Transaction>
    message_wrapper: MessageWrapper,
    mempool: MempoolRef,
    storage: SharedStore
}

impl Executor
{
    pub fn new(mempool: MempoolRef, storage: SharedStore, message_wrapper: MessageWrapper) -> Self
    {
        let (task_sender, task_receiver) = mpsc::channel();
        Executor
        {
            task_sender,
            task_receiver,
            message_wrapper,
            mempool,
            storage,
        }
    }

    pub fn get_sender(&self) -> Sender<Task>
    {
        self.task_sender.clone()
    }

    pub fn run(&mut self)
    {
        loop
        {
            if let Ok(task) = self.task_receiver.recv()
            {
                info!("task received, it is {:?}", task);
                match task
                {
                    Task::SignBlock() => self.sign_block(),
                    Task::CreateExampleTransaction(value) => self.create_example_transaction(&value)
                }
            }
        } 
    }

    fn create_example_transaction(&mut self, value_string: &String)
    {
        let _value = value_string.parse::<u64>().unwrap();
        let transaction = Transaction {
            version: 0,
            inputs: vec![TransactionInput {
                previous_output: OutPoint {
                    hash: "fff7f7881a8099afa6940d42d1e7f6362bec38171ea3edf433541db4e4ad969f".into(),
                    index: 0,
                },
                script_sig: "4830450221008b9d1dc26ba6a9cb62127b02742fa9d754cd3bebf337f7a55d114c8e5cdd30be022040529b194ba3f9281a99f2b1c0a19c0489bc22ede944ccf4ecbab4cc618ef3ed01".into(),
                sequence: 0xffffffee,
                script_witness: vec![],
            }, TransactionInput {
                previous_output: OutPoint {
                    hash: "ef51e1b804cc89d182d279655c3aa89e815b1b309fe287d9b2b55d57b90ec68a".into(),
                    index: 1,
                },
                script_sig: "".into(),
                sequence: 0xffffffff,
                script_witness: vec![
                    "304402203609e17b84f6a7d30c80bfa610b5b4542f32a8a0d5447a12fb1366d7f01cc44a0220573a954c4518331561406f90300e8f3358f51928d43c212a8caed02de67eebee01".into(),
                    "025476c2e83188368da1ff3e292e7acafcdb3566bb0ad253f62fc70f07aeee6357".into(),
                ],
            }],
            outputs: vec![TransactionOutput {
                value: 0x0000000006b22c20,
                script_pubkey: "76a9148280b37df378db99f66f85c95a783a76ac7a6d5988ac".into(),
            }, TransactionOutput {
                value: 0x000000000d519390,
                script_pubkey: "76a9143bde42dbee7e4dbe6a21b2d50ce2f0167faa815988ac".into(),
            }],
            lock_time: 0x0000_0011,
        };

        let tx = Tx { transaction: transaction.clone() };
        self.message_wrapper.wrap(&tx);
        
        let mut mempool = self.mempool.write().unwrap();
        mempool.transactions.push(transaction);
    }

    fn sign_block(&mut self)
    {
        let current_time = SystemTime::now();
        let time_since_the_epoch = current_time.duration_since(UNIX_EPOCH).expect("Time went backwards");

        let header = BlockHeader {
            version: 1,
            previous_header_hash: self.storage.best_block().hash,
            merkle_root_hash: DHash256::default().finish(),
            time: time_since_the_epoch.as_secs() as u32,
            bits: 5.into(),
            nonce: 6,
        };
        let mempool = self.mempool.write().unwrap();
        let mut block = Block::new(header, mempool.transactions.clone());
        
        //recalculate merkle root
        block.block_header.merkle_root_hash = block.witness_merkle_root();

        let block_message = BlockMessage { block };
        self.message_wrapper.wrap(&block_message);
    }
}