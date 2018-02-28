use std::sync::mpsc::{self, Sender, Receiver};
use chain::bytes::Bytes;
use message::MessageHeader;
use message::{Error, Payload, types, deserialize_payload};
use message::common::{InventoryType, InventoryVector};

use params::info::NETWORK_INFO;
use service::Service;
use crypto::checksum;
use db::SharedStore;
use mempool::MempoolRef;
use script::{verify_script, TransactionInputSigner, SignatureVersion, TransactionSignatureChecker};
use script::{ScriptWitness, VerificationFlags};
use script::Error as ScriptError;
use chain::Transaction;
use responder::ResponderTask;
use network::{PeerAndBytes, PeerIndex};

pub struct MessageHandler
{
    pub mempool: MempoolRef,
    pub network_data_receiver: Receiver<PeerAndBytes>,
    pub store: SharedStore,
    pub network_responder: Sender<ResponderTask>
}

impl MessageHandler
{
    pub fn new(mempool: MempoolRef, store: SharedStore, network_responder: Sender<ResponderTask>) -> Self
    {
        let (network_data_sender, network_data_receiver) = mpsc::channel();

        MessageHandler {
            mempool,
            network_data_receiver,
            store,
            network_responder
        }
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

    fn on_get_blocks(&self, peer: PeerIndex, message: types::GetBlocks)
    {
        self.network_responder.send(ResponderTask::GetBlocks(peer, message)).unwrap();
    }

    fn on_inv(&self, peer_index: PeerIndex, message: types::Inv)
    {
		let unknown_inventory: Vec<_> = message.inventory.into_iter()
			.filter(|item| {
				match item.inv_type {
					// check that transaction is unknown to us
					InventoryType::MessageTx => self.store.transaction(&item.hash).is_none(),
					InventoryType::MessageBlock => self.store.block_number(&item.hash).is_none(),   //check is block is known
					// we never ask for merkle blocks && we never ask for compact blocks
					InventoryType::MessageCompactBlock | InventoryType::MessageFilteredBlock
						| InventoryType::MessageWitnessBlock | InventoryType::MessageWitnessFilteredBlock
						| InventoryType::MessageWitnessTx => false,
					// unknown inventory type
					InventoryType::Error => {
						error!("Provided unknown inventory type {:?}", item.hash);
						false
					}
				}
			})
			.collect();

		// if everything is known => ignore this message
		if unknown_inventory.is_empty() {
			trace!(target: "sync", "Ignoring inventory message from peer#{} as all items are known", peer_index);
			return;
		}

		// ask for unknown items
		let message = types::GetData::with_inventory(unknown_inventory);
        self.network_responder.send(ResponderTask::GetData(peer_index, message)).unwrap();
    }

    fn on_message(&self, peer: PeerIndex, header: MessageHeader, payload: &[u8]) -> Result<(), Error>
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
        else if header.command == types::GetBlocks::command()
        {
            let message: types::GetBlocks = try!(deserialize_payload(payload, 0));
			self.on_get_blocks(peer, message);
        }
        else if header.command == types::Inv::command()
        {
            let message: types::Inv = try!(deserialize_payload(payload, 0));
			self.on_inv(peer, message);
        }
        Ok(())
    }
}

impl Service for MessageHandler
{
    type Item = PeerAndBytes;

    fn run(&mut self)
    {
        loop
        {
            if let Ok(peer_and_bytes) = self.network_data_receiver.recv()
            {
                let bytes = peer_and_bytes.bytes;
                let peer = peer_and_bytes.peer;
                //TODO check boundaries
                let data_start = 24;
                let info = NETWORK_INFO;
                match MessageHeader::deserialize(&bytes[ 0..data_start ], info.magic)
                {
                    Ok(header) => {
                        let data_end = data_start + header.len as usize;
                        let data = &bytes[ data_start..data_end ];
                        if let Err(err) = self.on_message(peer, header, data)
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