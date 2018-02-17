use std::sync::mpsc::{self, Sender, Receiver};
use chain::bytes::Bytes;
use message::MessageHeader;
use message::{Error, Payload, types, deserialize_payload};

use params::info::NETWORK_INFO;
use service::Service;
use crypto::checksum;
use db::SharedStore;
use mempool::MempoolRef;

pub struct MessageHandler
{
    mempool: MempoolRef,
    
    network_data_sender: Sender<Bytes>,
    network_data_receiver: Receiver<Bytes>,

    store: SharedStore
}

impl MessageHandler
{
    pub fn new(mempool: MempoolRef, store: SharedStore) -> Self
    {
        let (network_data_sender, network_data_receiver) = mpsc::channel();
        MessageHandler {
                    mempool,
                    network_data_sender,
                    network_data_receiver,
                    store
        }
    }

    fn on_transaction(&self, message: types::Tx)
    {
        info!("received transaction message {:?}", message);
		let mut mempool = self.mempool.write().unwrap();
        mempool.insert(message.transaction);
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
    fn get_sender(&self) -> Sender<Self::Item>
    {
        self.network_data_sender.clone()
    }

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
        } 
    }
}