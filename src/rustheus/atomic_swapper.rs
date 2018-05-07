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
use wallet::Wallet;
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

const secretSize: usize = 32;

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
    Redeem(),
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
    contractP2SH:   H256,
    contractTxHash: H256,
    contractTx:     Transaction,
    contractFee:    u64,
    refundTx:       Transaction,
    refundFee:      u64,
}

// AtomicSwapDataPushes houses the data pushes found in atomic swap contracts.
struct AtomicSwapDataPushes {
	RecipientHash160: H160,
	RefundHash160:    H160,
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
}

impl AtomicSwapper {
    pub fn new(
        acceptor: AcceptorRef,
        transaction_helper: TransactionHelperRef,
        cpupool: CpuPool,    
        message_wrapper: MessageWrapper,
        task_receiver: Receiver<Task>,  
    ) -> Self {
        AtomicSwapper {
            acceptor,
            transaction_helper,
            cpupool,
            message_wrapper,
            task_receiver
        }
    }

    pub fn run(&mut self) {
        loop {
            if let Ok(task) = self.task_receiver.recv() {
                match task {
                    Task::Initiate(address, amount) => self.initiate(address, amount),
                    Task::Participate(address, amount, secret) => self.participate(address, amount, secret),
                    Task::Redeem() => self.redeem(),
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
        let mut secret: [u8; secretSize] = [0u8; secretSize];
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
        println!("Contract ({}):", contract.contractP2SH);
        println!("{:?}\n", contract.contract);
        
        println!("Contract transaction ({}):", contract.contractTxHash);
        println!("{:?}\n", contract.contractTx);

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
    fn redeem(&self, ) {
        unimplemented!();
    }
    fn extract_secret(&self, transaction: H256, secret: H256) {
        unimplemented!();
    }
    fn audit_contract(&self, contract: Bytes, raw_contract_transaction: Bytes) {
        let contractHash160 = dhash160(&contract);

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
                let script: Script = output.script_pubkey.clone().into();
                let destinations = script.extract_destinations().unwrap_or(vec![]);
                destinations.iter().any(|address| address.hash == contractHash160)
            });

        let output = match output {
            Some(output) => output,
            None => {
                error!("transaction does not contain the contract output");
                return;
            }
        };

        let pushes =  match extractAtomicSwapDataPushes(0, contract) {
            Ok(pushes) => pushes,
            Err(err) => {
                error!("contract is not an atomic swap script recognized by this tool. Reason {:?}", err);
                return;
            }
        };

        if pushes.SecretSize as usize != secretSize {
            error!("contract specifies strange secret size {}", pushes.SecretSize);
            return;
        }

        let network = Network::Mainnet; 

        let contractAddr = Address {
            hash: contractHash160,
            network: network,
            kind: AddressType::P2SH,
        };
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

        println!("Contract address:        {}", contractAddr);
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
            let reachedAt = locktime - time_since_the_epoch;
            let reachedAt = reachedAt.as_secs();
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
        //TODO real new wallet address
        let mut refund_wallet = Wallet::new();
        let refund_address_hash = refund_wallet.new_keypair().hash;

        let contract = atomicSwapContract(refund_address_hash, args.them,
            args.locktime, args.secret_hash);

        let contract = contract.to_bytes();

        let contractP2SH = dhash256(&contract[..]);
        let contractP2SHPkScript = ScriptBuilder::build_p2wsh(&contractP2SH);

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
            contractP2SH,
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
		.push_num(secretSize.into())
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

	if pops.len() != 20 {
		return Err(PushExtractionError::NotAtomicSwapScript);
	}

    let pops2 = Opcode::from_u8(pops[2]).ok_or(PushExtractionError::MalformedAtomicSwapScript)?;
    let pops11 = Opcode::from_u8(pops[11]).ok_or(PushExtractionError::MalformedAtomicSwapScript)?;

	let isAtomicSwap = pops[0] == Opcode::OP_IF as u8 &&
		pops[1] == Opcode::OP_SIZE as u8 &&
		pops2.is_within_op_n() && //TODO what is canonical push?
		pops[3] == Opcode::OP_EQUALVERIFY as u8 &&
		pops[4] == Opcode::OP_SHA256 as u8 &&
		pops[5] == Opcode::OP_PUSHBYTES_32 as u8 &&
		pops[6] == Opcode::OP_EQUALVERIFY as u8 &&
		pops[7] == Opcode::OP_DUP as u8 &&
		pops[8] == Opcode::OP_HASH160 as u8 &&
		pops[9] == Opcode::OP_PUSHBYTES_20 as u8 &&
		pops[10] == Opcode::OP_ELSE as u8 &&
		pops11.is_within_op_n() && //TODO what is canonical push?
		pops[12] == Opcode::OP_CHECKLOCKTIMEVERIFY as u8 &&
		pops[13] == Opcode::OP_DROP as u8 &&
		pops[14] == Opcode::OP_DUP as u8 &&
		pops[15] == Opcode::OP_HASH160 as u8 &&
		pops[16] == Opcode::OP_PUSHBYTES_20 as u8 &&
		pops[17] == Opcode::OP_ENDIF as u8 &&
		pops[18] == Opcode::OP_EQUALVERIFY as u8 &&
		pops[19] == Opcode::OP_CHECKSIG as u8;
	if !isAtomicSwap {
		return Err(PushExtractionError::NotAtomicSwapScript);
	}

    //let pushes: AtomicSwapDataPushes = 
	let SecretHash = pops[5].into();
	let RecipientHash160 = pops[9].into();
	let RefundHash160 = pops[16].into();

    let secret_size = Num::from_slice(&pops[2..2], true, 5)?;
    let locktime = Num::from_slice(&pops[11..11], true, 5)?;

	Ok(AtomicSwapDataPushes {
        SecretHash,
        RecipientHash160,
        RefundHash160,
        SecretSize: secret_size.into(),
        LockTime: locktime.into(),
    })
}