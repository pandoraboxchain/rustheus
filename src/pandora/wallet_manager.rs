use keys::generator::{Random, Generator};
use keys::network::Network;
use keys::Address;
use wallet_manager_tasks::Task;
use std::sync::mpsc::{self, Sender, Receiver};
use service::Service;
use chain::Transaction;
use message_wrapper::MessageWrapper;
use message::types::Tx;
use mempool::MempoolRef;

//temp
use chain::{TransactionInput, TransactionOutput, OutPoint};


pub struct WalletManager
{
    receiver: Receiver<Task>,
    sender: Sender<Task>,
    mempool: MempoolRef,
    wrapper: MessageWrapper
}

impl WalletManager
{
    pub fn new(mempool: MempoolRef, wrapper: MessageWrapper) -> Self
    {
        let (sender, receiver) = mpsc::channel();
        WalletManager
        {
            sender,
            receiver,
            mempool,
            wrapper
        }
    }

    fn create_wallet(&self)
    {
        let generator = Random::new(Network::Mainnet);
        match generator.generate()
        {
            Ok(keypair) =>
            {
                info!("got keypair {}", keypair);
                info!("address is {}", keypair.address());
            } 
            Err(error) => error!("error generating keypair {:?}", error)
        }
    }

    fn send_cash(&self, to: Address, amount: u32)
    {
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
        self.wrapper.wrap(&tx);
        
        let mut mempool = self.mempool.write().unwrap();
        mempool.transactions.push(transaction);
    }
}

impl<'a> Service for WalletManager
{
    type Item = Task;
    fn get_sender(&self) -> Sender<Self::Item>
    {
        self.sender.clone()
    }

    fn run(&mut self)
    {
        loop
        {
            if let Ok(task) = self.receiver.recv()
            {
                info!("wallet task received, it is {:?}", task);
                match task
                {
                    Task::CreateWallet() => self.create_wallet(),
                    Task::SendCash(to, amount) => self.send_cash(to, amount)
                }
            }
        } 
    }
}

