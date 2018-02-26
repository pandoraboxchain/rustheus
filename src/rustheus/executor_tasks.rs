use keys::Address;
use primitives::hash::H256;

#[derive(Debug, PartialEq)]
pub enum Task
{
	SignBlock(Address),
	GetTransactionMeta(H256)
}