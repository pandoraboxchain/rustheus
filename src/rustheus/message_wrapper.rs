use message::Payload;
use std::sync::mpsc::{Sender};
use chain::bytes::Bytes;
use params::info::NETWORK_INFO;
use ser::SERIALIZE_TRANSACTION_WITNESS;
use message::Message;

pub struct MessageWrapper
{
    network_channel: Sender<Bytes>
}

impl MessageWrapper
{
    pub fn new(network_channel: Sender<Bytes>) -> Self
    {
        MessageWrapper
        {
            network_channel
        }
    }

    pub fn wrap<T>(&self, payload: &T) where T: Payload //TODO use moving here instead of borrowing
    {
        let info = NETWORK_INFO;
		let message = Message::with_flags(info.magic, info.version, payload, SERIALIZE_TRANSACTION_WITNESS).expect("failed to create outgoing message");
        self.network_channel.send(message.as_ref().into()).unwrap();
    }
}