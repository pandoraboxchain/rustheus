use network::PeerIndex;
use db::SharedStore;
use message::{types, common};
use std::sync::mpsc::{self, Sender, Receiver};
use primitives::hash::H256;
use message_wrapper::MessageWrapper;

type BlockHeight = u32;

#[derive(Debug, PartialEq)]
pub enum ResponderTask {
	GetBlocks(PeerIndex, types::GetBlocks),
}

pub struct Responder {
    pub task_receiver: Receiver<ResponderTask>,
    pub message_wrapper: MessageWrapper,
    pub storage: SharedStore
}

impl Responder {
    pub fn new(storage: SharedStore, message_wrapper: MessageWrapper) -> (Self, Sender<ResponderTask>) {
        let (task_sender, task_receiver) = mpsc::channel();

        let responder = Responder {
            task_receiver,
            storage,
            message_wrapper
        };
        (responder, task_sender)
    } 

    pub fn run(&self) {
        loop {
            match self.task_receiver.recv() {
                Err(_) => break,
                Ok(task) => {
                    match task {
                        ResponderTask::GetBlocks(peer_index, message) => self.respond_get_blocks(peer_index, message),
                    }
                }
            }
        }
    }

    fn respond_get_blocks(&self, peer_index: PeerIndex, message: types::GetBlocks) {
        if let Some(block_height) = self.locate_best_common_block(&message.hash_stop, &message.block_locator_hashes) {
            let inventory: Vec<_> = (block_height + 1..block_height + 1 + (500 as BlockHeight))
                .map(|block_height| self.storage.block_hash(block_height))
                .take_while(Option::is_some)
                .map(Option::unwrap)
                .take_while(|block_hash| block_hash != &message.hash_stop)
                .map(common::InventoryVector::block)
                .collect();
            // empty inventory messages are invalid according to regtests, while empty headers messages are valid
            if !inventory.is_empty() {
                trace!(target: "sync", "'getblocks' response to peer#{} is ready with {} hashes", peer_index, inventory.len());
                let inventory_msg = types::Inv::with_inventory(inventory);
                self.message_wrapper.wrap(&inventory_msg);
                //self.executor.execute(Task::Inventory(peer_index, types::Inv::with_inventory(inventory)));
            } else {
                trace!(target: "sync", "'getblocks' request from peer#{} is ignored as there are no new blocks for peer", peer_index);
            }
        } else {
            //self.peers.misbehaving(peer_index, "Got 'getblocks' message without known blocks");
            return;
        }
    }

    fn locate_best_common_block(&self, hash_stop: &H256, locator: &[H256]) -> Option<BlockHeight> {
		for block_hash in locator.iter().chain(&[hash_stop.clone()]) {
			if let Some(block_number) = self.storage.block_number(block_hash) {
				return Some(block_number);
			}

			// block with this hash is definitely not in the main chain (block_number has returned None)
			// but maybe it is in some fork? if so => we should find intersection with main chain
			// and this would be our best common block
			let mut block_hash = block_hash.clone();
			loop {
				let block_header = match self.storage.block_header(block_hash.into()) {
					None => break,
					Some(block_header) => block_header,
				};

				if let Some(block_number) = self.storage.block_number(&block_header.previous_header_hash) {
					return Some(block_number);
				}

				block_hash = block_header.previous_header_hash;
			}
		}

		None
	}
}

