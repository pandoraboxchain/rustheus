use chain::Transaction;
use chain::constants::SEQUENCE_LOCKTIME_DISABLE_FLAG;
use db::SharedStore;
use keys::{Address, Private};
use memory_pool::MemoryPoolRef;
use message::types::Tx;
use script::{Builder, Script, SighashBase, SignatureVersion, TransactionInputSigner};
use service::Service;
use std::sync::mpsc::Receiver;
use sync::MessageWrapper;
use wallet::Wallet;

//temp
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
    wallet: Wallet,
    storage: SharedStore,
}

impl WalletManager {
    pub fn new(
        mempool: MemoryPoolRef,
        storage: SharedStore,
        receiver: Receiver<Task>,
        wrapper: MessageWrapper,
    ) -> Self {
        let wallet = Wallet::new();
        WalletManager {
            receiver,
            mempool,
            wrapper,
            wallet,
            storage,
        }
    }

    fn create_wallet(&mut self) {
        self.wallet.new_keypair();
    }

    fn load_from_key(&mut self, private: Private) {
        match self.wallet.add_keypair_from_private(private) {
            Ok(_) => {}
            Err(err) => error!("Failed to create wallet from private: {}", err),
        }
    }

    fn calculate_balance(&self) {
        if self.wallet.keys.is_empty() {
            error!("No wallet was created or loaded. Use `walletcreate` or `walletload` to create one.");
            return;
        }
        let wallet = &self.wallet;

        let user_address_hash = wallet.keys[0].address().hash;
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

    //TODO needs refactoring so it not just returns in case of error
    fn send_cash(&self, recipient: Address, amount: u64) {
        if self.wallet.keys.is_empty() {
            error!("No wallet was created or loaded. Use `walletcreate` or `walletfromkey` to create one.");
            return;
        }

        let wallet = &self.wallet;
        let user_address_hash = wallet.keys[0].address().hash;
        let unspent_out_points = self.storage
            .transaction_with_output_address(&user_address_hash);
        if unspent_out_points.is_empty() {
            error!("No unspent outputs found. I.e. no money on current address");
            return;
        }
        let unspent_outputs: Vec<TransactionOutput> = unspent_out_points
            .iter()
            .map(|out_point| self.storage.transaction_output(out_point, 0).unwrap())
            .collect();

        if unspent_outputs[0].value < amount {
            error!("Not enough money on first input.");
            return;
        }

        let mut outputs: Vec<TransactionOutput> = vec![
            TransactionOutput {
                value: amount,
                script_pubkey: Builder::build_p2wpkh(&recipient.hash).to_bytes(),
            },
        ];

        let leftover = unspent_outputs[0].value - amount;
        if leftover > 0
        //if something left, send it back
        //TODO create new address and send it there
        {
            outputs.push(TransactionOutput {
                value: leftover,
                script_pubkey: Builder::build_p2wpkh(&user_address_hash).to_bytes(),
            });
        }

        let version = 0;
        let lock_time = 0;

        let transaction = Transaction {
            version,
            inputs: vec![
                TransactionInput {
                    previous_output: unspent_out_points[0].clone(),
                    script_sig: Default::default(),
                    sequence: SEQUENCE_LOCKTIME_DISABLE_FLAG,
                    script_witness: vec![],
                },
            ],
            outputs: outputs.clone(),
            lock_time,
        };

        let signer: TransactionInputSigner = transaction.into();
        let prevout_script = Script::new(unspent_outputs[0].script_pubkey.clone());
        let prevout_witness = prevout_script.parse_witness_program();
        if prevout_witness == None {
            error!("Cannot parse previous output witness");
            return;
        }

        let prevout_witness_version = prevout_witness.unwrap().0;
        if prevout_witness_version != 0 {
            error!("Previous output witness version is too high and cannot be handled");
            return;
        }

        let prevout_witness_program = prevout_witness.unwrap().1;
        let script_pubkey = Builder::build_p2pkh(&prevout_witness_program.into());

        let signed_input = signer.signed_input(
            &wallet.keys[0],
            /*input_index*/ 0,
            unspent_outputs[0].value,
            &script_pubkey,
            SignatureVersion::WitnessV0,
            SighashBase::All.into(),
        );

        let signed_transaction = Transaction {
            version,
            inputs: vec![signed_input],
            outputs,
            lock_time,
        };

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
