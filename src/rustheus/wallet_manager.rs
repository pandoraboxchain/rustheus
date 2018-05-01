use chain::constants::SEQUENCE_LOCKTIME_DISABLE_FLAG;
use chain::{OutPoint, Transaction};
//use chain_builder::TransactionBuilder;
use db::SharedStore;
use keys::{Address, Private};
use memory_pool::MemoryPoolRef;
use message::types::Tx;
use script::{Builder, Script, SighashBase, SignatureVersion, TransactionInputSigner};
use service::Service;
use std::sync::mpsc::Receiver;
use sync::MessageWrapper;
use wallet::{Wallet, WalletRef};
use transaction_helper::TransactionHelperRef;
use chain::{TransactionInput, TransactionOutput};

#[derive(Debug, PartialEq)]
pub enum Task {
    CreateWallet(),
    SendCash(Address, u64),
    LoadWallet(Private),
    CalculateBalance(),
}

pub struct WalletManager {
    receiver: Receiver<Task>,
    mempool: MemoryPoolRef,
    wrapper: MessageWrapper,
    wallet: WalletRef,
    storage: SharedStore,
    transaction_helper: TransactionHelperRef,
}

impl WalletManager {
    pub fn new(
        mempool: MemoryPoolRef,
        storage: SharedStore,
        receiver: Receiver<Task>,
        wrapper: MessageWrapper,
        wallet: WalletRef,
        transaction_helper: TransactionHelperRef,    
    ) -> Self {
        WalletManager {
            receiver,
            mempool,
            wrapper,
            storage,
            wallet,
            transaction_helper,
        }
    }

    fn create_wallet(&self) {
        self.wallet.write().new_keypair();
    }

    fn load_from_key(&self, private: Private) {
        match self.wallet.write().add_keypair_from_private(private) {
            Ok(_) => {}
            Err(err) => error!("Failed to create wallet from private: {}", err),
        }
    }

    fn calculate_balance(&self) {
        if self.wallet.read().keys.is_empty() {
            error!("No wallet was created or loaded. Use `walletcreate` or `walletload` to create one.");
            return;
        }
        let wallet = &self.wallet;

        let user_address_hash = wallet.read().keys[0].address().hash;
        let out_points = self.storage
            .transaction_with_output_address(&user_address_hash);
        println!("out_points len is {}", out_points.len());
        for out_point in out_points.iter() {
            println!("out_point is {:?}", out_point);
        }
        let balance = out_points
            .iter()
            .map(|out_point| self.storage.transaction_output(out_point, 0).unwrap())
            .fold(0, |credit, outpoint| credit + outpoint.value);

        info!("wallet balance is {}", balance);
    }

    //TODO needs refactoring so it not just returns in case of error
    fn send_cash(&self, recipient: Address, amount: u64) {
        if self.wallet.read().keys.is_empty() {
            error!("No wallet was created or loaded. Use `walletcreate` or `walletfromkey` to create one.");
            return;
        }

        let transaction = Transaction {
            version: 0,
            inputs: vec![],
            outputs: vec![
                TransactionOutput {
                    value: amount,
                    script_pubkey: Builder::build_p2wpkh(&recipient.hash).to_bytes(),
                },
            ],
            lock_time: 0,
        };

        //TODO pattern match returned results
        let funded_transaction = self.transaction_helper.fund_transaction(transaction).unwrap();
        let signed_transaction = self.transaction_helper.sign_transaction(funded_transaction).unwrap();

        let hash = signed_transaction.hash();
        if self.mempool.read().contains(&hash) {
            error!("Exact same transaction already exists in mempool");
            return;
        }

        let tx = Tx {
            transaction: signed_transaction.clone(),
        };
        self.wrapper.broadcast(&tx);

        let mut mempool = self.mempool.write();
        mempool.insert_verified(signed_transaction.into());
    }
}

impl Service for WalletManager {
    type Item = Task;

    fn run(&mut self) {
        loop {
            if let Ok(task) = self.receiver.recv() {
                match task {
                    Task::CreateWallet() => self.create_wallet(),
                    Task::LoadWallet(private) => self.load_from_key(private),
                    Task::CalculateBalance() => self.calculate_balance(),
                    Task::SendCash(to, amount) => self.send_cash(to, amount),
                }
            } else {
                debug!("wallet manager thread ended");
                break;
            }
        }
    }
}
