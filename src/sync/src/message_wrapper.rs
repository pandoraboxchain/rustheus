use message::Payload;
use std::sync::mpsc::Sender;
use params::NetworkParams;
use ser::SERIALIZE_TRANSACTION_WITNESS;
use message::Message;
use p2p::{PeerAndBytes, PeerIndex, XorName};

#[derive(Clone)]
pub struct MessageWrapper {
    network_params: NetworkParams,
    network_channel: Sender<PeerAndBytes>,
}

impl MessageWrapper {
    pub fn new(network_params: NetworkParams, network_channel: Sender<PeerAndBytes>, ) -> Self {
        MessageWrapper {
            network_params,
            network_channel,
        }
    }

    pub fn broadcast<T>(&self, payload: &T)
    where
        T: Payload, //TODO use moving here instead of borrowing
    {
        let version = 0;
        let message = Message::with_flags(
            self.network_params.magic(),
            version,
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
        let version = 0;
        let message = Message::with_flags(
            self.network_params.magic(),
            version,
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
