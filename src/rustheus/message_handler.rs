use std::sync::mpsc::{Receiver, Sender};
use message::MessageHeader;
use message::{deserialize_payload, types, Error, Payload};
use message::common::InventoryType;

use service::Service;
use crypto::checksum;
use db::SharedStore;
use chain::IndexedBlock;
use responder::ResponderTask;
use network::{PeerAndBytes, PeerIndex};
use message_wrapper::MessageWrapper;
use acceptor::Task as AcceptorTask;
use params::NetworkParams;

pub struct MessageHandler {
    network_data_receiver: Receiver<PeerAndBytes>,
    store: SharedStore,
    network_responder: Sender<ResponderTask>,
    acceptor_sender: Sender<AcceptorTask>,
    message_wrapper: MessageWrapper,
    params: NetworkParams,
}

impl MessageHandler {
    pub fn new(
        store: SharedStore,
        network_data_receiver: Receiver<PeerAndBytes>,
        network_responder: Sender<ResponderTask>,
        acceptor_sender: Sender<AcceptorTask>,
        message_wrapper: MessageWrapper,
        params: NetworkParams,
    ) -> Self {

        MessageHandler {
            store,
            network_data_receiver,
            network_responder,
            acceptor_sender,
            message_wrapper,
            params,
        }
    }

    fn on_transaction(&self, message: types::Tx) {
        self.acceptor_sender
            .send(AcceptorTask::TryAcceptTransaction(message.transaction))
            .unwrap();
    }

    fn on_block(&self, message: types::Block) {
        self.acceptor_sender
            .send(AcceptorTask::TryAcceptBlock(message.block))
            .unwrap();
    }

    fn on_inv(&self, peer_index: PeerIndex, message: types::Inv) {
        let unknown_inventory: Vec<_> = message
            .inventory
            .into_iter()
            .filter(|item| {
                match item.inv_type {
                    // check that transaction is unknown to us
                    InventoryType::MessageTx => self.store.transaction(&item.hash).is_none(),
                    InventoryType::MessageBlock => self.store.block_number(&item.hash).is_none(), //check is block is known
                    // we never ask for merkle blocks && we never ask for compact blocks
                    InventoryType::MessageCompactBlock
                    | InventoryType::MessageFilteredBlock
                    | InventoryType::MessageWitnessBlock
                    | InventoryType::MessageWitnessFilteredBlock
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

        trace!(target: "handler", "unknown items are {:?}", unknown_inventory);

        // ask for unknown items
        let message = types::GetData::with_inventory(unknown_inventory);
        self.message_wrapper.send(peer_index, &message);
    }

    //TODO maybe move following methods to separate handler
    fn on_get_blocks(&self, peer: PeerIndex, message: types::GetBlocks) {
        self.network_responder
            .send(ResponderTask::GetBlocks(peer, message))
            .unwrap();
    }

    fn on_get_data(&self, peer: PeerIndex, message: types::GetData) {
        self.network_responder
            .send(ResponderTask::GetData(peer, message))
            .unwrap();
    }

    fn on_message(
        &self,
        peer: PeerIndex,
        header: MessageHeader,
        payload: &[u8],
    ) -> Result<(), Error> {
        if checksum(payload) != header.checksum {
            return Err(Error::InvalidChecksum);
        }

        if header.command == types::Tx::command() {
            let message: types::Tx = try!(deserialize_payload(payload, 0));
            trace!(target: "handler", "received tx {:?}", message);
            self.on_transaction(message);
        } else if header.command == types::Block::command() {
            let message: types::Block = try!(deserialize_payload(payload, 0));
            trace!(target: "handler", "received block {:?}", message);
            self.on_block(message);
        } else if header.command == types::GetBlocks::command() {
            let message: types::GetBlocks = try!(deserialize_payload(payload, 0));
            trace!(target: "handler", "received getblocks {:?}", message);
            self.on_get_blocks(peer, message);
        } else if header.command == types::Inv::command() {
            let message: types::Inv = try!(deserialize_payload(payload, 0));
            trace!(target: "handler", "received inv {:?}", message);
            self.on_inv(peer, message);
        } else if header.command == types::GetData::command() {
            let message: types::GetData = try!(deserialize_payload(payload, 0));
            trace!(target: "handler", "received getdata {:?}", message);
            self.on_get_data(peer, message);
        }
        Ok(())
    }
}

impl Service for MessageHandler {
    type Item = PeerAndBytes;

    fn run(&mut self) {
        loop {
            if let Ok(peer_and_bytes) = self.network_data_receiver.recv() {
                let bytes = peer_and_bytes.bytes;
                let peer = peer_and_bytes.peer;
                //TODO check boundaries
                let data_start = 24;
                match MessageHeader::deserialize(&bytes[0..data_start], self.params.magic()) {
                    Ok(header) => {
                        let data_end = data_start + header.len as usize;
                        let data = &bytes[data_start..data_end];
                        if let Err(err) = self.on_message(peer, header, data) {
                            error!(
                                "Unable to deserialize received message body. Reason: {:?}",
                                err
                            )
                        }
                    }
                    Err(err) => error!(
                        "Unable to deserialize received message header. Reason: {:?}",
                        err
                    ),
                }
            } else {
                debug!("message handler thread finished");
                break;
            }
        }
    }
}
