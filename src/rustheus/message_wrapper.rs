use message::Payload;
use std::sync::mpsc::Sender;
use params::info::NETWORK_INFO;
use ser::SERIALIZE_TRANSACTION_WITNESS;
use message::Message;
use network::{PeerAndBytes, PeerIndex};
use routing::XorName;

pub struct MessageWrapper {
    network_channel: Sender<PeerAndBytes>,
}

impl MessageWrapper {
    pub fn new(network_channel: Sender<PeerAndBytes>) -> Self {
        MessageWrapper { network_channel }
    }

    pub fn broadcast<T>(&self, payload: &T)
    where
        T: Payload, //TODO use moving here instead of borrowing
    {
        let info = NETWORK_INFO;
        let message = Message::with_flags(
            info.magic,
            info.version,
            payload,
            SERIALIZE_TRANSACTION_WITNESS,
        ).expect("failed to create outgoing message");
        let peer_and_bytes = PeerAndBytes {
            peer: XorName::default(),
            bytes: message.as_ref().into(),
        };
        self.network_channel.send(peer_and_bytes).unwrap();
    }

    pub fn send<T>(&self, peer: PeerIndex, payload: &T)
    where
        T: Payload, //TODO use moving here instead of borrowing
    {
        let info = NETWORK_INFO;
        let message = Message::with_flags(
            info.magic,
            info.version,
            payload,
            SERIALIZE_TRANSACTION_WITNESS,
        ).expect("failed to create outgoing message");
        let peer_and_bytes = PeerAndBytes {
            peer,
            bytes: message.as_ref().into(),
        };
        self.network_channel.send(peer_and_bytes).unwrap();
    }
}
