use chain::{Block, Transaction};

#[derive(Debug, PartialEq)]
pub enum Message {
	Block(Block),
	Transaction(Transaction),
}
