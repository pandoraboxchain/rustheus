#![allow(non_snake_case)]

use script::Builder as ScriptBuilder;
use chain_builder::TransactionBuilder;
use keys::generator::Random;
use primitives::hash::{H256, H160};
use keys::{Address, AddressHash};
use sync::{AcceptorRef, MessageWrapper};
use chain::bytes::Bytes;
use chain::{Transaction};
use crypto::{dhash160, dhash256};
use std::time::{SystemTime, UNIX_EPOCH};
use wallet::WalletRef;
use message::types::Tx;
use transaction_helper::{TransactionHelperRef, SignError, FundError};
use std::sync::mpsc::Receiver;
use ser::{deserialize, serialize, Reader};
use script::Error as ScriptError;
use script::{Script, Opcode, Num};
use chain::constants::LOCKTIME_THRESHOLD;
use keys::Network;
use keys::Type as AddressType;
use futures::prelude::*;
use futures_cpupool::CpuPool;
use std::time::Duration;

const SECRET_SIZE: usize = 32;

#[derive(Debug)]
pub enum ContractError
{
    FundError(FundError),
    SignError(SignError),
}

#[derive(Debug)]
pub enum PushExtractionError
{
    NoPushes,
    NotAtomicSwapScript,
    MalformedAtomicSwapScript,
    ScriptError(ScriptError),
}

impl From<ScriptError> for PushExtractionError {
    fn from(err: ScriptError) -> PushExtractionError {
        PushExtractionError::ScriptError(err)
    }
}

impl From<SignError> for ContractError {
    fn from(err: SignError) -> ContractError {
        ContractError::SignError(err)
    }
}

impl From<FundError> for ContractError {
    fn from(err: FundError) -> ContractError {
        ContractError::FundError(err)
    }
}

#[derive(Debug, PartialEq)]
pub enum Task {
    //atomic swaps
    Initiate(Address, u64),
    Participate(Address, u64, H256),
    Redeem(Bytes, Bytes, Bytes),
    ExtractSecret(H256, H256),
    AuditContract(Bytes, Bytes),
    //TODO refund
}

struct ContractArgs {
    them:       AddressHash,
    amount:     u64,
    locktime:   u32,
    secret_hash: H256,
}

struct BuiltContract {
    contract:       Bytes,
    contractP2WSH:  H256,
    contractTxHash: H256,
    contractTx:     Transaction,
    contractFee:    u64,
    refundTx:       Transaction,
    refundFee:      u64,
}

// AtomicSwapDataPushes houses the data pushes found in atomic swap contracts.
struct AtomicSwapDataPushes {
	RecipientHash160: AddressHash,
	RefundHash160:    AddressHash,
	SecretHash:       H256,
	SecretSize:       i64,  //TODO why is this signed?
	LockTime:         i64,  //TODO why is this signed?
}

pub struct AtomicSwapper {
    acceptor: AcceptorRef,
    cpupool: CpuPool,
    message_wrapper: MessageWrapper, 
    transaction_helper: TransactionHelperRef,
    task_receiver: Receiver<Task>,
    wallet: WalletRef,
}

impl AtomicSwapper {
    pub fn new(
        acceptor: AcceptorRef,
        transaction_helper: TransactionHelperRef,
        cpupool: CpuPool,    
        message_wrapper: MessageWrapper,
        task_receiver: Receiver<Task>,
        wallet: WalletRef, 
    ) -> Self {
        AtomicSwapper {
            acceptor,
            transaction_helper,
            cpupool,
            message_wrapper,
            task_receiver,
            wallet,
        }
    }

    pub fn run(&mut self) {
        loop {
            if let Ok(task) = self.task_receiver.recv() {
                match task {
                    Task::Initiate(address, amount) => self.initiate(address, amount),
                    Task::Participate(address, amount, secret) => self.participate(address, amount, secret),
                    Task::Redeem(contract, contract_transaction, secret) => self.redeem(contract, contract_transaction, secret),
                    Task::ExtractSecret(transaction, secret) => self.extract_secret(transaction, secret),
                    Task::AuditContract(contract, contract_transaction) => self.audit_contract(contract, contract_transaction),
                }
            } else {
                break;
            }
        }
    }

    fn initiate(&mut self, address: Address, amount: u64) {
        //TODO check if correct network
		//let mut secret = [u8; 32];
        let mut secret: [u8; SECRET_SIZE] = [0u8; SECRET_SIZE];
        if let Err(_) = Random::generate_bytes(&mut secret[..]) {
            error!("Could not generate bytes for secret");
            return;
        }
        let secret_hash = dhash256(&secret);
        
        let current_time = SystemTime::now();
        let time_since_the_epoch = current_time
            .duration_since(UNIX_EPOCH)
            .expect("Time went backwards");

        let locktime = time_since_the_epoch.as_secs() + (48 * 60 * 60); //48 hours

        println!("Secret:      {:?}", secret);
        println!("Secret hash: {}\n", secret_hash);

        let contract = self.buildContract(ContractArgs {
            them:       address.hash,
            amount:     amount,
            locktime:   locktime as u32,    //TODO check if u32 is suitable
            secret_hash: secret_hash,
        });

        let contract = match contract {
            Ok(built_contract) => built_contract,
            Err(err) => {
                error!("Failed to build contract. Reason: {:?}", err);
                return;
            }
        };

        let refundTxHash = contract.refundTx.hash();
        //TODO fee calculation
        //let contractFeePerKb = calcFeePerKb(b.contractFee, b.contractTx.SerializeSize())
        //let refundFeePerKb = calcFeePerKb(b.refundFee, b.refundTx.SerializeSize())

        //println!("Contract fee: %v (%0.8f BTC/kB)\n", b.contractFee, contractFeePerKb);
        //println!("Refund fee:   %v (%0.8f BTC/kB)\n\n", b.refundFee, refundFeePerKb);
        println!("Contract ({}):", contract.contractP2WSH);
        println!("{:?}\n", contract.contract);
        
        println!("Contract transaction ({}):", contract.contractTxHash);
        println!("{:?}\n", serialize(&contract.contractTx));

        println!("Refund transaction ({}):\n", refundTxHash);
        println!("{:?}\n", contract.refundTx);

        let message_wrapper = self.message_wrapper.to_owned();
        let task = self.acceptor.async_accept_transaction(contract.contractTx.clone())
            .map(move |transaction| message_wrapper.broadcast(&Tx::with_transaction(transaction)));

        let _ = self.cpupool.spawn(task);
    }
    
    fn participate(&self, address: Address, amount: u64, secret: H256) {
        unimplemented!();
    }
    fn extract_secret(&self, transaction: H256, secret: H256) {
        unimplemented!();
    }

    fn redeem(&self, contract: Bytes, raw_contract_transaction: Bytes, secret: Bytes) {
        let contractHash256 = dhash256(&contract);        
        let pushes =  match extractAtomicSwapDataPushes(0, contract.clone()) {
            Ok(pushes) => pushes,
            Err(err) => {
                error!("Cannot parse contract. Reason {:?}", err);
                return;
            }
        };

        let network = Network::Mainnet; //TODO check for network correctness

        Address {
            hash: pushes.RecipientHash160,
            network: network,
            kind: AddressType::P2PKH,
        };

       let raw_transaction_data: Vec<u8> = raw_contract_transaction.into();
		let transaction: Transaction = match deserialize(Reader::new(&raw_transaction_data)) {
            Ok(transaction) => transaction,
            Err(err) => {
                error!("Cannot deserialize transaction: {:?}", err);
                return;
            }
        };

        let found_output = transaction.outputs.iter()
            .enumerate()
            .find(|(_, output)| {
                if output.script_pubkey.len() == 34 {       //TODO use script address instead of script hash here
                    let script_unlocking_hash = &output.script_pubkey[2..];
                    return script_unlocking_hash == &contractHash256[..];
                }
                false
            });

        let (output_index, output) = match found_output {
            Some((output_index, output)) => (output_index, output),
            None => {
                error!("Transaction does not contain the contract output");
                return;
            }
        };

        let recipientAddr = self.wallet.write().new_keypair();
        let wallet = self.wallet.read();
        let key = wallet.find_keypair_with_public_hash(&recipientAddr.hash).unwrap(); //HACK

        let outScript = ScriptBuilder::build_p2wpkh(&recipientAddr.hash);

        //feePerKb, minFeePerKb, err := getFeePerKb(c)
        let (feePerKb, minFeePerKb) = (0,0);

        let mut redeemTx: Transaction = TransactionBuilder::with_output_and_pubkey(0, outScript.to_bytes())
            .set_input(&transaction, output_index as u32)
            .set_lock_time(pushes.LockTime as u32)
            .into();

        //redeemSize := estimateRedeemSerializeSize(cmd.contract, redeemTx.TxOut)
        //fee := txrules.FeeForSerializeSize(feePerKb, redeemSize)
        let fee = 0;
        //redeemTx.TxOut[0].Value = cmd.contractTx.TxOut[contractOut].Value - int64(fee)
        //if txrules.IsDustOutput(redeemTx.TxOut[0], minFeePerKb) {
        //    return fmt.Errorf("redeem output value of %v is dust", btcutil.Amount(redeemTx.TxOut[0].Value))
        //}
        let (redeemSig, redeemPubKey) = self.transaction_helper.create_signature_for_input(&redeemTx, 0, output.value, contract.clone().into(), &key);
        let redeemSigScript = redeemP2WSHContract(contract, redeemSig, redeemPubKey, secret);
        
        redeemTx.inputs[0].script_witness = redeemSigScript;

        let redeemTxHash = redeemTx.hash();
        //redeemFeePerKb := calcFeePerKb(fee, redeemTx.SerializeSize()) //TODO

        println!("Redeem transaction {}:", &redeemTxHash);
        println!("Size {} bytes", serialize(&redeemTx).len());

        //TODO if verify flag was specified let script run and check that everything is ok
        // if verify {
        //     e, err := txscript.NewEngine(cmd.contractTx.TxOut[contractOutPoint.Index].PkScript,
        //         redeemTx, 0, txscript.StandardVerifyFlags, txscript.NewSigCache(10),
        //         txscript.NewTxSigHashes(redeemTx), cmd.contractTx.TxOut[contractOut].Value)
        //     if err != nil {
        //         panic(err)
        //     }
        //     err = e.Execute()
        //     if err != nil {
        //         panic(err)
        //     }
        // }

        let message_wrapper = self.message_wrapper.to_owned();
        let task = self.acceptor.async_accept_transaction(redeemTx.clone())
            .map(move |transaction| message_wrapper.broadcast(&Tx::with_transaction(transaction)));
        
        let _ = self.cpupool.spawn(task);
    }
    
    fn audit_contract(&self, contract: Bytes, raw_contract_transaction: Bytes) {
        let contractHash256 = dhash256(&contract);

		let raw_transaction_data: Vec<u8> = raw_contract_transaction.into();
		let transaction: Transaction = match deserialize(Reader::new(&raw_transaction_data)) {
            Ok(transaction) => transaction,
            Err(err) => {
                error!("Cannot deserialize transaction: {:?}", err);
                return;
            }
        };

        let output = transaction.outputs.iter()
            .find(|output| {
                if output.script_pubkey.len() == 34 {       //TODO use script address instead of script hash here
                    let script_unlocking_hash = &output.script_pubkey[2..];
                    return script_unlocking_hash == &contractHash256[..];
                }
                false
            });

        let output = match output {
            Some(output) => output,
            None => {
                error!("Transaction does not contain the contract output");
                return;
            }
        };

        let pushes =  match extractAtomicSwapDataPushes(0, contract) {
            Ok(pushes) => pushes,
            Err(err) => {
                error!("Cannot parse contract. Reason {:?}", err);
                return;
            }
        };

        if pushes.SecretSize as usize != SECRET_SIZE {
            error!("Contract specifies strange secret size {}", pushes.SecretSize);
            return;
        }

        let network = Network::Mainnet; //TODO check for network correctness

        //TODO bech32
//        let contractAddr = Address {
//            hash: contractHash160,
//            network: network,
//            kind: AddressType::P2SH,
//        };
        let recipientAddr = Address {
            hash: pushes.RecipientHash160,
            network: network,
            kind: AddressType::P2PKH,
        };
        let refundAddr = Address {
            hash: pushes.RefundHash160,
            network: network,
            kind: AddressType::P2PKH,
        };

        println!("Contract address:        {}", contractHash256);
        println!("Contract value:          {}", output.value);
        println!("Recipient address:       {}", recipientAddr);
        println!("Author's refund address: {}\n", refundAddr);

        println!("Secret hash: {}\n", pushes.SecretHash);

        if pushes.LockTime >= LOCKTIME_THRESHOLD as i64 {
            let current_time = SystemTime::now();
            let locktime = Duration::from_secs(pushes.LockTime as u64);
            let time_since_the_epoch = current_time
                .duration_since(UNIX_EPOCH)
                .expect("System time went backwards");
            println!("Locktime: {}", pushes.LockTime);
            let reachedAt: i128 = locktime.as_secs() as i128 - time_since_the_epoch.as_secs() as i128;
            if reachedAt > 0 {
                println!("Locktime reached in {} seconds", reachedAt);
            } else {
                println!("Contract refund time lock has expired");
        }
        } else {
            println!("Locktime: block {}", pushes.LockTime);
        }
    }

    fn buildContract(&mut self, args: ContractArgs) -> Result<BuiltContract, ContractError> {
        let refund_address_hash = self.wallet.write().new_keypair().hash;

        let contract = atomicSwapContract(refund_address_hash, args.them,
            args.locktime, args.secret_hash);

        let contract = contract.to_bytes();

        let contractP2WSH = dhash256(&contract[..]);
        let contractP2SHPkScript = ScriptBuilder::build_p2wsh(&contractP2WSH);

        //TODO fee calculation
        let (feePerKb, minFeePerKb) = (0,0);

        let transaction: Transaction = TransactionBuilder::with_output_and_pubkey(args.amount, contractP2SHPkScript.to_bytes()).into();

        let funded_transaction = self.transaction_helper.fund_transaction(transaction)?;
        let contractTx = self.transaction_helper.sign_transaction(funded_transaction)?;

        let contractFee = 0u64;

        // let task = self.wallet_manager.spawn(fund_and_sign);
        //TODO build a refund transaction
        //refundTx, refundFee, err := buildRefund(c, contract, contractTx, feePerKb, minFeePerKb)
        let refundTx = TransactionBuilder::default().into();
        let refundFee = 0u64;

        let contractTxHash = contractTx.hash();
        Ok(BuiltContract {
            contract,
            contractP2WSH,
            contractTxHash,
            contractTx,
            contractFee,
            refundTx,
            refundFee,
        })
    }
}

// atomicSwapContract returns an output script that may be redeemed by one of
// two signature scripts:
//
//   <their sig> <their pubkey> <initiator secret> 1
//
//   <my sig> <my pubkey> 0
//
// The first signature script is the normal redemption path done by the other
// party and requires the initiator's secret.  The second signature script is
// the refund path performed by us, but the refund can only be performed after
// locktime.
fn atomicSwapContract(pkhMe: H160, pkhThem: H160, locktime: u32, secretHash: H256) -> Script {
    let script = ScriptBuilder::default()
	.push_opcode(Opcode::OP_IF) // Normal redeem path
	
		// Require initiator'ss secret to be a known length that the redeeming
		// party can audit.  This is used to prevent fraud attacks between two
		// currencies that have different maximum data sizes.
		.push_opcode(Opcode::OP_SIZE)
		.push_num(SECRET_SIZE.into())
		.push_opcode(Opcode::OP_EQUALVERIFY)

		// Require initiator's secret to be known to redeem the output.
		.push_opcode(Opcode::OP_SHA256)
		.push_data(&*secretHash)
		.push_opcode(Opcode::OP_EQUALVERIFY)

		// Verify their signature is being used to redeem the output.  This
		// would normally end with Opcode::OP_EQUALVERIFY Opcode::OP_CHECKSIG but this has been
		// moved outside of the branch to save a couple bytes.
		.push_opcode(Opcode::OP_DUP)
		.push_opcode(Opcode::OP_HASH160)
		.push_data(&*pkhThem)
	
    .push_opcode(Opcode::OP_ELSE) // Refund path
	
		// Verify locktime and drop it off the stack (which is not done by
		// CLTV).
		.push_num(locktime.into())
		.push_opcode(Opcode::OP_CHECKLOCKTIMEVERIFY)
		.push_opcode(Opcode::OP_DROP)

		// Verify our signature is being used to redeem the output.  This would
		// normally end with Opcode::OP_EQUALVERIFY Opcode::OP_CHECKSIG but this has been moved
		// outside of the branch to save a couple bytes.
		.push_opcode(Opcode::OP_DUP)
		.push_opcode(Opcode::OP_HASH160)
		.push_data(&*pkhMe)
	
	.push_opcode(Opcode::OP_ENDIF)

	// Complete the signature check.
	.push_opcode(Opcode::OP_EQUALVERIFY)
	.push_opcode(Opcode::OP_CHECKSIG)
    .into_script();

	script
}

fn extractAtomicSwapDataPushes(_version: u16, pkScript: Bytes) -> Result<AtomicSwapDataPushes,PushExtractionError> {
	let pops = pkScript;

    let script: Script = pops.clone().into();
	if script.opcodes().count() != 20 {
		return Err(PushExtractionError::NotAtomicSwapScript);
	}

    let mut opcodes = script.opcodes();

    //TODO think of a better way of checking script correctness
	let isAtomicSwap = opcodes.next().unwrap()? == Opcode::OP_IF &&
        opcodes.next().unwrap()? == Opcode::OP_SIZE &&
        opcodes.next().unwrap()?.is_simple_push() && //TODO check for minimal push here maybe?
		opcodes.next().unwrap()? == Opcode::OP_EQUALVERIFY &&
		opcodes.next().unwrap()? == Opcode::OP_SHA256 &&
		opcodes.next().unwrap()? == Opcode::OP_PUSHBYTES_32 &&
		opcodes.next().unwrap()? == Opcode::OP_EQUALVERIFY &&
		opcodes.next().unwrap()? == Opcode::OP_DUP &&
		opcodes.next().unwrap()? == Opcode::OP_HASH160 &&
		opcodes.next().unwrap()? == Opcode::OP_PUSHBYTES_20 &&
		opcodes.next().unwrap()? == Opcode::OP_ELSE &&
        opcodes.next().unwrap()?.is_simple_push() && //TODO check for minimal push here maybe?
		opcodes.next().unwrap()? == Opcode::OP_CHECKLOCKTIMEVERIFY &&
		opcodes.next().unwrap()? == Opcode::OP_DROP &&
		opcodes.next().unwrap()? == Opcode::OP_DUP &&
		opcodes.next().unwrap()? == Opcode::OP_HASH160 &&
		opcodes.next().unwrap()? == Opcode::OP_PUSHBYTES_20 &&
		opcodes.next().unwrap()? == Opcode::OP_ENDIF &&
		opcodes.next().unwrap()? == Opcode::OP_EQUALVERIFY &&
		opcodes.next().unwrap()? == Opcode::OP_CHECKSIG;
	if !isAtomicSwap {
		return Err(PushExtractionError::NotAtomicSwapScript);
	}

    //let pushes: AtomicSwapDataPushes = 
	let SecretHash = script.iter().nth(5).unwrap()?.data.ok_or(PushExtractionError::MalformedAtomicSwapScript)?.into();
	let RecipientHash160 = script.iter().nth(9).unwrap()?.data.ok_or(PushExtractionError::MalformedAtomicSwapScript)?.into();
	let RefundHash160 = script.iter().nth(16).unwrap()?.data.ok_or(PushExtractionError::MalformedAtomicSwapScript)?.into();;

    let secret_size_slice = script.iter().nth(2).unwrap()?.data.ok_or(PushExtractionError::MalformedAtomicSwapScript)?;
    let locktime_slice = script.iter().nth(11).unwrap()?.data.ok_or(PushExtractionError::MalformedAtomicSwapScript)?;

    let secret_size = Num::from_slice(secret_size_slice, true, 5)?;
    let locktime = Num::from_slice(locktime_slice, true, 5)?;

	Ok(AtomicSwapDataPushes {
        SecretHash,
        RecipientHash160,
        RefundHash160,
        SecretSize: secret_size.into(),
        LockTime: locktime.into(),
    })
}

// redeemP2SHContract returns the signature script to redeem a contract output
// using the redeemer's signature and the initiator's secret.  This function
// assumes P2WSH and appends the contract as the final data push.
fn redeemP2WSHContract(contract: Bytes, sig: Bytes, pubkey: Bytes, secret: Bytes) -> Vec<Bytes> {
	vec![sig, pubkey, secret, vec![1].into(), contract]
}

// refundP2SHContract returns the signature script to refund a contract output
// using the contract author's signature after the locktime has been reached.
// This function assumes P2WSH and appends the contract as the final data push.
fn refundP2WSHContract(contract: Bytes, sig: Bytes, pubkey: Bytes) -> Vec<Bytes> {
    vec![sig, pubkey, vec![0].into(), contract]
}
