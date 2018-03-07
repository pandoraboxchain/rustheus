use keys::{Address, Private};
use wallet_manager_tasks::Task;
use std::sync::mpsc::{self, Receiver, Sender};
use service::Service;
use chain::Transaction;
use message_wrapper::MessageWrapper;
use message::types::Tx;
use memory_pool::MemoryPoolRef;
use wallet::Wallet;
use db::SharedStore;
use script::{Builder, Script, SighashBase, SignatureVersion, TransactionInputSigner};
use chain::constants::SEQUENCE_LOCKTIME_DISABLE_FLAG;

//temp
use chain::{TransactionInput, TransactionOutput};

pub struct WalletManager {
    receiver: Receiver<Task>,
    mempool: MemoryPoolRef,
    wrapper: MessageWrapper,
    wallets: Vec<Wallet>,
    storage: SharedStore,
}

impl WalletManager {
    pub fn new(
        mempool: MemoryPoolRef,
        storage: SharedStore,
        wrapper: MessageWrapper,
    ) -> (Self, Sender<Task>) {
        let (sender, receiver) = mpsc::channel();
        let wallets = vec![];
        let wallet_manager = WalletManager {
            receiver,
            mempool,
            wrapper,
            wallets,
            storage,
        };
        (wallet_manager, sender)
    }

    fn create_wallet(&mut self) {
        let wallet = Wallet::new().unwrap();
        self.wallets.push(wallet);
    }

    fn load_from_key(&mut self, private: Private) {
        match Wallet::from_private(private) {
            Ok(wallet) => self.wallets.push(wallet),
            Err(err) => error!("failed to create wallet from private: {}", err),
        }
    }

    fn calculate_balance(&self) {
        if self.wallets.is_empty() {
            error!("No wallet was created or loaded. Use `walletcreate` or `walletload` to create one.");
            return;
        }
        let wallet = &self.wallets[0];

        let user_address_hash = wallet.keys.address().hash;
        let out_points = self.storage
            .transaction_with_output_address(&user_address_hash);
        println!("out_points len is {}", out_points.len());
        for out_point in out_points.iter() {
            println!("out_point is {:?}", out_point);
        }
        let balance = out_points
            .iter()
            .map(|out_point| self.storage.transaction_output(out_point, 0).unwrap())
            .fold(0, |credit, output| credit + output.value);

        info!("wallet balance is {}", balance);
    }

    fn send_cash(&self, recipient: Address, amount: u64) {
        if self.wallets.is_empty() {
            error!("No wallet was created or loaded. Use `walletcreate` or `walletfromkey` to create one.");
            return;
        }

        let wallet = &self.wallets[0];
        let user_address_hash = wallet.keys.address().hash;
        let unspent_out_points = self.storage
            .transaction_with_output_address(&user_address_hash);
        if unspent_out_points.is_empty() {
            error!("No unspent outputs found. I.e. no money on current address"); //TODO
            return;
        }
        let unspent_outputs: Vec<TransactionOutput> = unspent_out_points
            .iter()
            .map(|out_point| self.storage.transaction_output(out_point, 0).unwrap())
            .collect();

        if unspent_outputs[0].value < amount {
            error!("Not enough money on first input."); //TODO
            return;
        }

        let mut outputs: Vec<TransactionOutput> = vec![
            TransactionOutput {
                value: amount,
                script_pubkey: Builder::build_p2pkh(&recipient.hash).to_bytes(),
            },
        ];

        let leftover = unspent_outputs[0].value - amount;
        if leftover > 0
        //if something left, send it back
        {
            outputs.push(TransactionOutput {
                value: leftover,
                script_pubkey: Builder::build_p2pkh(&user_address_hash).to_bytes(),
            });
        }

        let transaction = Transaction {
            version: 0,
            inputs: vec![
                TransactionInput {
                    previous_output: unspent_out_points[0].clone(),
                    script_sig: unspent_outputs[0].script_pubkey.clone(),
                    sequence: SEQUENCE_LOCKTIME_DISABLE_FLAG,
                    script_witness: vec![],
                },
            ],
            outputs: outputs.clone(),
            lock_time: 0xffffffff,
        };

        let signer: TransactionInputSigner = transaction.into();
        let prevout_script_pubkey: Script = unspent_outputs[0].script_pubkey.clone().into();
        //TODO find out if we need to use witness hashing here
        //NOTE input_amount is unused if signing by original procedure
        let signed_input = signer.signed_input(
            &wallet.keys,
            /*input_index*/ 0,
            /*input_amount*/ 0,
            &prevout_script_pubkey,
            SignatureVersion::Base,
            SighashBase::All.into(),
        );

        let signed_transaction = Transaction {
            version: 0,
            inputs: vec![signed_input],
            outputs: outputs,
            lock_time: 0xffffffff,
        };

        let tx = Tx {
            transaction: signed_transaction.clone(),
        };
        self.wrapper.broadcast(&tx);

        let mut mempool = self.mempool.write().unwrap();
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
