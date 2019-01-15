use hash::H256;
use chain::PaymentTransaction;

#[derive(Debug, PartialEq, Serializable, Deserializable)]
pub struct BlockTransactions {
	pub blockhash: H256,
	pub transactions: Vec<PaymentTransaction>,
}
