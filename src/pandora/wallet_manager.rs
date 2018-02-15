use keys::{Address, Private};
use wallet_manager_tasks::Task;
use std::sync::mpsc::{self, Sender, Receiver};
use service::Service;
use chain::Transaction;
use message_wrapper::MessageWrapper;
use message::types::Tx;
use mempool::MempoolRef;
use wallet::Wallet;
use db::SharedStore;
//temp
use chain::{TransactionInput, TransactionOutput, OutPoint};


pub struct WalletManager
{
    receiver: Receiver<Task>,
    sender: Sender<Task>,
    mempool: MempoolRef,
    wrapper: MessageWrapper,
    wallets: Vec<Wallet>,
    storage: SharedStore
}

impl WalletManager
{
    pub fn new(mempool: MempoolRef, storage: SharedStore, wrapper: MessageWrapper) -> Self
    {
        let (sender, receiver) = mpsc::channel();
        let wallets = vec![];
        WalletManager
        {
            sender,
            receiver,
            mempool,
            wrapper,
            wallets,
            storage
        }
    }

    fn create_wallet(&mut self)
    {
        let wallet = Wallet::new().unwrap();
        self.wallets.push(wallet);
    }

    fn load_from_key(&mut self, private: Private)
    {
        match Wallet::from_private(private)
        {
            Ok(wallet) => self.wallets.push(wallet),
            Err(err) => error!("failed to create wallet from private: {}", err)
        }
    }

    fn calculate_balance(&self)
    {
        if self.wallets.is_empty()
        {
            error!("No wallet was created or loaded. Use `walletcreate` or `walletfromkey` to create one.");
            return;
        }  
        let wallet = &self.wallets[0];

        let address_hash = wallet.keys.address().hash;
        let outputs = self.storage.transaction_with_output_address(&address_hash);
        println!("outputs len is {}", outputs.len());
        for output in outputs.iter()
        {
            println!("output is {:?}", output);
        }
        let balance = outputs.iter().fold(0, |credit, outputs| credit + outputs.value);
        info!("wallet balance is {}", balance);
    }

    fn send_cash(&self, to: Address, amount: u64)
    {
        if self.wallets.is_empty()
        {
            error!("No wallet was created or loaded. Use `walletcreate` or `walletfromkey` to create one.");
            return;
        }  
        let wallet = &self.wallets[0];

        let address_string = to.to_string();

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
            }],
            outputs: vec![TransactionOutput {
                value: amount,
                script_pubkey: address_string.as_bytes().into()
            }],
            lock_time: 0,
        };

        let tx = Tx { transaction: transaction.clone() };
        self.wrapper.wrap(&tx);
        
        let mut mempool = self.mempool.write().unwrap();
        mempool.transactions.push(transaction);
    }
}

impl Service for WalletManager
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
                match task
                {
                    Task::CreateWallet() => self.create_wallet(),
                    Task::LoadWallet(private) => self.load_from_key(private), //TODO simpilify this
                    Task::CalculateBalance() => self.calculate_balance(),
                    Task::SendCash(to, amount) => self.send_cash(to, amount)
                }
            }
        } 
    }
}

