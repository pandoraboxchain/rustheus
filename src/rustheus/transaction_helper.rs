use chain_pan::constants::SEQUENCE_LOCKTIME_DISABLE_FLAG;
use chain_pan::{OutPoint};
use db::{TransactionUtxoProvider, TransactionOutputProvider};
use keys::{Private, KeyPair};
use script::{Builder, Script, SighashBase, SignatureVersion, TransactionInputSigner};
use wallet::WalletRef;
use chain_pan::{TransactionInput, TransactionOutput};
use std::sync::Arc;
use memory_pool::UtxoAndOutputProvider;
use primitives::bytes::Bytes;
use chain_pan::PaymentTransaction;

pub type TransactionHelperRef = Arc<TransactionHelper>;

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
    FundError(FundError),
}

impl From<FundError> for SignError {
    fn from(err: FundError) -> SignError {
        SignError::FundError(err)
    }
}

pub struct TransactionHelper {
    utxo_provider: UtxoAndOutputProvider,
    wallet: WalletRef,
}

impl TransactionHelper {
    pub fn new(
        utxo_provider: UtxoAndOutputProvider,
        wallet: WalletRef,
    ) -> Self {
        TransactionHelper {
            utxo_provider,
            wallet,
        }
    }

    //TODO seek for spent outputs in mempool
    fn get_unspent_out_points(&self) -> Vec<OutPoint> {
        self.wallet
            .read()
            .keys
            .iter()
            .flat_map(|keypair| 
                self.utxo_provider
                    .transaction_with_output_address(&keypair.address().hash))
            .collect()
    }

    //TODO accept fee
    pub fn fund_transaction(&self, transaction: PaymentTransaction) -> Result<PaymentTransaction, FundError> {
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
            let output = self.utxo_provider.transaction_output(&out_point, 0).unwrap();
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

        //TODO create option to return leftovers to the same address
        if inputs_sum > needed_amount {
            let new_address = self.wallet.write().new_keypair();
            let leftover = TransactionOutput {
                value: inputs_sum - needed_amount,
                script_pubkey: Builder::build_p2wpkh(&new_address.hash).to_bytes(),
            };
            outputs.push(leftover);
        } else if inputs_sum < needed_amount {
            return Err(FundError::NotEnoughFunds);
        }

        Ok(PaymentTransaction {
            version: 0,
            inputs,
            outputs,
            lock_time: 0,
        })
    }

    // createSig creates and returns the serialized raw signature and compressed
    // pubkey for a transaction input signature
    pub fn create_signature_for_input(&self, transaction: &PaymentTransaction, input_index: usize,
		input_amount: u64, script: Script,	keys: &KeyPair) -> (Bytes, Bytes) {
        let signer: TransactionInputSigner = transaction.clone().into();        
        
        signer.compute_signature_for_input(keys,
            input_index,
            input_amount,
            &script,
            SignatureVersion::WitnessV0,
            SighashBase::All.into())
    }

    pub fn sign_transaction(&self, transaction: PaymentTransaction) -> Result<PaymentTransaction, SignError> {
        let signer: TransactionInputSigner = transaction.clone().into();
        let signed_inputs: Result<Vec<_>, _> = transaction
            .inputs
            .into_iter()
            .enumerate()
            .map(|(i, input)| self.sign_input(input, i, &signer))
            .collect();

        match signed_inputs {
            Err(err) => Err(err),
            Ok(signed_inputs) => Ok(PaymentTransaction {
                version: transaction.version,
                inputs: signed_inputs,
                outputs: transaction.outputs,
                lock_time: transaction.lock_time,
            }),
        }
    }

    pub fn sign_input(
        &self,
        input: TransactionInput,
        input_index: usize,
        signer: &TransactionInputSigner,
    ) -> Result<TransactionInput, SignError> {
        let prevout = self.utxo_provider.transaction_output(&input.previous_output, 0);
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
        let wallet = self.wallet.read();
        let keys = wallet
            .find_keypair_with_public_hash(&prevout_witness_program.into());
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
}
