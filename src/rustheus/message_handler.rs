use std::sync::mpsc::{self, Sender, Receiver};
use chain::bytes::Bytes;
use message::MessageHeader;
use message::{Error, Payload, types, deserialize_payload};

use params::info::NETWORK_INFO;
use service::Service;
use crypto::checksum;
use db::SharedStore;
use mempool::MempoolRef;
use script::{verify_script, TransactionInputSigner, SignatureVersion, TransactionSignatureChecker};
use script::{ScriptWitness, VerificationFlags};
use script::Error as ScriptError;
use chain::Transaction;

pub struct MessageHandler
{
    mempool: MempoolRef,
    network_data_receiver: Receiver<Bytes>,
    store: SharedStore
}

impl MessageHandler
{
    pub fn new(mempool: MempoolRef, store: SharedStore) -> (Self, Sender<Bytes>)
    {
        let (network_data_sender, network_data_receiver) = mpsc::channel();

        let message_handler = MessageHandler {
                    mempool,
                    network_data_receiver,
                    store
        };
        (message_handler, network_data_sender)
    }

    //TODO move it to appropriate file
    //TODO make it check not only [0] input
    fn verify_transaction(&self, transaction: &Transaction) -> Result<(), ScriptError>
    {
        let input = &transaction.inputs[0];

        let prev_output = self.store.transaction_output(&input.previous_output, 0).expect("No such previous output in received transaction found. Discarding");

        let signer: TransactionInputSigner = transaction.clone().into();
        let checker = TransactionSignatureChecker {
            signer: signer,
            input_index: 0,
            input_amount: 0,
        };

        let script_sig = input.script_sig.clone().into();
        let script_pubkey = prev_output.script_pubkey.into();

        verify_script(&script_sig, &script_pubkey, &ScriptWitness::default(), &VerificationFlags::default(), &checker,SignatureVersion::Base)
    }
    //TODO check inputs other than [0]
    fn on_transaction(&self, message: types::Tx)
    {
        info!("received transaction message {:?}", message);

        let verification_result = self.verify_transaction(&message.transaction);

        match verification_result
        {
            Err(err) => error!("Failed to accept transaction to mempool. {}", err),
            Ok(_) => {
                let mut mempool = self.mempool.write().unwrap();
                mempool.insert(message.transaction);       
            }
        }
	}
    
    fn on_block(&self, message: types::Block)
    {
        let block = message.block;
        let hash = block.hash().clone();
        let transactions = block.transactions.clone();
        match self.store.insert(block.into()) {
            Ok(_) => match self.store.canonize(&hash) {
                    Ok(_) => {
                        info!("Block inserted and canonized with hash {}", hash);
                        let mut mempool = self.mempool.write().unwrap();
                        mempool.remove_transactions(transactions);
                    },
                    Err(err) => error!("Cannot canonize received block due to {:?}", err)
                }
            Err(err) => error!("Cannot insert received block due to {:?}", err)
        }

    }

    fn on_message(&self, header: MessageHeader, payload: &[u8]) -> Result<(), Error>
    {
        if checksum(&payload) != header.checksum 
        {
            return Err(Error::InvalidChecksum);
		}

        if header.command == types::Tx::command()
        {
			let message: types::Tx = try!(deserialize_payload(payload, 0));
			self.on_transaction(message);
		}
        else if header.command == types::Block::command()
        {
            let message: types::Block = try!(deserialize_payload(payload, 0));
			self.on_block(message);
        }
        Ok(())
    }
}

impl Service for MessageHandler
{
    type Item = Bytes;

    fn run(&mut self)
    {
        loop
        {
            if let Ok(bytes) = self.network_data_receiver.recv()
            {
                //TODO check boundaries
                let data_start = 24;
                let info = NETWORK_INFO;
                match MessageHeader::deserialize(&bytes[ 0..data_start ], info.magic)
                {
                    Ok(header) => {
                        let data_end = data_start + header.len as usize;
                        let data = &bytes[ data_start..data_end ];
                        if let Err(err) = self.on_message(header, data)
                        {
                            error!("Unable to deserialize received message body. Reason: {:?}", err)
                        }
                    }
                    Err(err) => error!("Unable to deserialize received message header. Reason: {:?}", err)
                }
            }
            else
            {
                debug!("message handler thread finished");
                break;
            }
        } 
    }
}