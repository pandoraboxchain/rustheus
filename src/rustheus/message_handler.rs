use std::sync::mpsc::{Receiver, Sender};
use message::MessageHeader;
use message::{deserialize_payload, types, Error, Payload};
use message::common::InventoryType;

use params::info::NETWORK_INFO;
use service::Service;
use crypto::checksum;
use db::SharedStore;
use memory_pool::MemoryPoolRef;
use memory_pool::MemoryPoolTransactionOutputProvider;
use chain::IndexedBlock;
use responder::ResponderTask;
use network::{PeerAndBytes, PeerIndex};
use message_wrapper::MessageWrapper;
use verification::BackwardsCompatibleChainVerifier as ChainVerifier;
use verification::{VerificationLevel, Verify};
use params::{ConsensusFork, ConsensusParams, NetworkParams};
use executor::Task as ExecutorTask;

pub struct MessageHandler {
    mempool: MemoryPoolRef,
    network_data_receiver: Receiver<PeerAndBytes>,
    store: SharedStore,
    network_responder: Sender<ResponderTask>,
    executor_sender: Sender<ExecutorTask>,
    message_wrapper: MessageWrapper,

    verifier: ChainVerifier,
}

impl MessageHandler {
    pub fn new(
        mempool: MemoryPoolRef,
        store: SharedStore,
        network_data_receiver: Receiver<PeerAndBytes>,
        network_responder: Sender<ResponderTask>,
        executor_sender: Sender<ExecutorTask>,
        message_wrapper: MessageWrapper,
    ) -> Self {
        let verifier = ChainVerifier::new(
            store.clone(),
            ConsensusParams::new(NetworkParams::Mainnet, ConsensusFork::NoFork),
        );

        MessageHandler {
            mempool,
            store,
            network_data_receiver,
            network_responder,
            executor_sender,
            message_wrapper,
            verifier,
        }
    }

    //TODO check inputs other than [0]
    fn on_transaction(&self, message: types::Tx) {
        let transaction = message.transaction;
        let hash = transaction.hash();
        if self.mempool.read().contains(&hash) {
            trace!(target: "handler", "Received transaction which already exists in mempool. Ignoring");
            return;
        }
        match MemoryPoolTransactionOutputProvider::for_transaction(
            self.store.clone(),
            &self.mempool,
            &transaction,
        ) {
            Err(e) => error!(
                "Can't accept transaction {} into mempool {:?}",
                transaction.hash(),
                e
            ),
            Ok(tx_output_provider) => {
                let height = self.store.best_block().number;
                match self.verifier.verify_mempool_transaction(
                    &tx_output_provider,
                    height,
                    /*time*/ 0,
                    &transaction,
                ) {
                    Ok(_) => {
                        // we have verified transaction, but possibly this transaction replaces
                        // existing transaction from memory pool
                        // => remove previous transactions before
                        let mut memory_pool = self.mempool.write();
                        for input in &transaction.inputs {
                            memory_pool.remove_by_prevout(&input.previous_output);
                        }
                        // now insert transaction itself
                        memory_pool.insert_verified(transaction.into());
                    }
                    Err(e) => error!(
                        "Can't accept transaction {} into mempool {:?}",
                        transaction.hash(),
                        e
                    ),
                }
            }
        };
    }

    fn on_block(&self, message: types::Block) {
        let block: IndexedBlock = message.block.into();
        match self.verifier.verify(VerificationLevel::Full, &block) {
            Ok(_) => self.executor_sender
                .send(ExecutorTask::AddVerifiedBlock(block))
                .unwrap(),
            Err(err) => error!("Invalid block received: {:?}", err),
        }
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
                let info = NETWORK_INFO;
                match MessageHeader::deserialize(&bytes[0..data_start], info.magic) {
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
