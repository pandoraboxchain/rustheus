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
use wallet::Wallet;

//temp
use chain::{TransactionInput, TransactionOutput};

#[derive(Debug)]
pub enum FundError {
    NoFunds,
    NotEnoughFunds,
}

#[derive(Debug)]
pub enum SignError {
    NoSuchPrevout,
    NoKeysToUnlockPrevout,
    PrevoutWitnessParseError,
    PrevoutWitnessVersionTooHigh,
}

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
            .fold(0, |credit, outpoint| credit + outpoint.value);

        info!("wallet balance is {}", balance);
    }

    fn get_unspent_out_points(&self) -> Vec<OutPoint> {
        self.wallet
            .keys
            .iter()
            .flat_map(|keypair| {
                self.storage
                    .transaction_with_output_address(&keypair.address().hash)
            })
            .collect()
    }

    //TODO accept fee
    fn fund_transaction(&mut self, transaction: Transaction) -> Result<Transaction, FundError> {
        let unspent_out_points = self.get_unspent_out_points();
        if unspent_out_points.is_empty() {
            return Err(FundError::NoFunds);
        }

        let needed_amount = transaction
            .outputs
            .iter()
            .fold(0, |acc, output| acc + output.value);

        let mut inputs: Vec<TransactionInput> = vec![];

        let mut inputs_sum = 0;
        for out_point in unspent_out_points {
            let output = self.storage.transaction_output(&out_point, 0).unwrap();
            let input = TransactionInput {
                previous_output: out_point,
                script_sig: Default::default(),
                sequence: SEQUENCE_LOCKTIME_DISABLE_FLAG, //TODO remove this for atomic swap probably
                script_witness: vec![],
            };
            inputs.push(input);

            inputs_sum += output.value;
            if inputs_sum >= needed_amount {
                break;
            }
        }

        let mut outputs = transaction.outputs.clone();

        if inputs_sum > needed_amount {
            let new_address = self.wallet.new_keypair();
            let leftover = TransactionOutput {
                value: inputs_sum - needed_amount,
                script_pubkey: Builder::build_p2wpkh(&new_address.hash).to_bytes(),
            };
            outputs.push(leftover);
        } else {
            return Err(FundError::NotEnoughFunds);
        }

        Ok(Transaction {
            version: 0,
            inputs,
            outputs,
            lock_time: 0,
        })
    }

    fn sign_transaction(&mut self, transaction: Transaction) -> Result<Transaction, SignError> {
        let signer: TransactionInputSigner = transaction.clone().into();
        let signed_inputs: Result<Vec<_>, _> = transaction
            .inputs
            .into_iter()
            .enumerate()
            .map(|(i, input)| self.sign_input(input, i, &signer))
            .collect();

        match signed_inputs {
            Err(err) => Err(err),
            Ok(signed_inputs) => Ok(Transaction {
                version: transaction.version,
                inputs: signed_inputs,
                outputs: transaction.outputs,
                lock_time: transaction.lock_time,
            }),
        }
    }

    fn sign_input(
        &self,
        input: TransactionInput,
        input_index: usize,
        signer: &TransactionInputSigner,
    ) -> Result<TransactionInput, SignError> {
        let prevout = self.storage.transaction_output(&input.previous_output, 0);
        if prevout.is_none() {
            return Err(SignError::NoSuchPrevout);
        }
        let prevout = prevout.unwrap();
        let prevout_script = Script::new(prevout.script_pubkey.clone());
        let prevout_witness = prevout_script.parse_witness_program();
        if prevout_witness == None {
            return Err(SignError::PrevoutWitnessParseError);
        }

        let prevout_witness_version = prevout_witness.unwrap().0;
        if prevout_witness_version != 0 {
            return Err(SignError::PrevoutWitnessVersionTooHigh);
        }

        let prevout_witness_program = prevout_witness.unwrap().1;
        let keys = self.wallet
            .find_keypair_with_public_hash(prevout_witness_program.into());
        if keys.is_none() {
            return Err(SignError::NoKeysToUnlockPrevout);
        }
        let keys = keys.unwrap();
        let script_pubkey = Builder::build_p2pkh(&prevout_witness_program.into());

        let signed_input = signer.signed_input(
            keys,
            input_index,
            prevout.value,
            &script_pubkey,
            SignatureVersion::WitnessV0,
            SighashBase::All.into(),
        );

        Ok(signed_input)
    }

    //TODO needs refactoring so it not just returns in case of error
    fn send_cash(&mut self, recipient: Address, amount: u64) {
        if self.wallet.keys.is_empty() {
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
        let funded_transaction = self.fund_transaction(transaction).unwrap();
        let signed_transaction = self.sign_transaction(funded_transaction).unwrap();

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
