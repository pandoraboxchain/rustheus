use chain::Transaction;
use std::sync::mpsc::{self, Sender, Receiver};
use chain::bytes::Bytes;
use message::MessageHeader;

use params::info::NETWORK_INFO;
use service::Service;

pub struct MessageHandler
{
    mempool_channel: Sender<Transaction>,
    
    network_data_sender: Sender<Bytes>,
    network_data_receiver: Receiver<Bytes>
}

impl MessageHandler
{
    pub fn new(mempool_channel: Sender<Transaction>) -> Self
    {
        let (network_data_sender, network_data_receiver) = mpsc::channel();
        MessageHandler {
                    mempool_channel,
                    network_data_sender,
                    network_data_receiver,
        }
    }

    // fn handle_transaction(data: &Vec<u8>)
    // {
    //     let deserialized = deserialize::<_, Transaction>(&data[..]);
    //     match deserialized
    //     {
    //         Ok(transaction) => {
    //             println!(" received transaction {:?}", transaction);
    //             mempool.push(transaction);
    //         }
    //         Err(_) => {}
    //     }
    // }
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
                let info = NETWORK_INFO;
                let header = MessageHeader::deserialize(&bytes[0..24], info.magic);
                println!("{:?}", header);      
            }
        } 
    }
}