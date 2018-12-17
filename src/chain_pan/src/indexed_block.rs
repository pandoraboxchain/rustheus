use std::cmp;
use hash::H256;
use ser::{Serializable, serialized_list_size, serialized_list_size_with_flags, deserialize, SERIALIZE_TRANSACTION_WITNESS};
use block::Block;
use transaction::Transaction;
use merkle_root::merkle_root;
use indexed_header::IndexedBlockHeader;
use indexed_transaction::IndexedTransaction;
use rustc_serialize::hex::FromHex;

#[derive(Debug, Clone, Deserializable)]
pub struct IndexedBlock {
    pub header: IndexedBlockHeader,
    pub transactions: Vec<Box<IndexedTransaction>>,
}

impl From<Block> for IndexedBlock {
    fn from(block: Block) -> Self {
        let Block { block_header, transactions } = block;

        IndexedBlock {
            header: block_header.into(),
            transactions: transactions.into_iter().map(Into::into).collect(),
        }
    }
}

impl IndexedBlock {
    pub fn new(header: IndexedBlockHeader, transactions: Vec<Box<IndexedTransaction>>) -> Self {
        IndexedBlock {
            header: header,
            transactions: transactions,
        }
    }

    pub fn hash(&self) -> &H256 {
        &self.header.hash
    }

    pub fn to_raw_block(self) -> Block {
        Block::new(self.header.raw, self.transactions.into_iter().map(|tx| tx.raw).collect())
    }

    pub fn size(&self) -> usize {
        let header_size = self.header.raw.serialized_size();
        let transactions = self.transactions.iter().map(|tx| &tx.raw).collect::<Vec<_>>();
        let txs_size = serialized_list_size::<Transaction, &Transaction>(&transactions);
        header_size + txs_size
    }

    pub fn size_with_witness(&self) -> usize {
        let header_size = self.header.raw.serialized_size();
        let transactions = self.transactions.iter().map(|tx| &tx.raw).collect::<Vec<_>>();
        let txs_size = serialized_list_size_with_flags::<Transaction, &Transaction>(&transactions, SERIALIZE_TRANSACTION_WITNESS);
        header_size + txs_size
    }

    pub fn merkle_root(&self) -> H256 {
        merkle_root(&self.transactions.iter().map(|tx| &tx.hash).collect::<Vec<&H256>>())
    }

    pub fn witness_merkle_root(&self) -> H256 {
        let hashes = match self.transactions.split_first() {
            None => vec![],
            Some((_, rest)) => {
                let mut hashes = vec![H256::from(0)];
                hashes.extend(rest.iter().map(|tx| tx.raw.witness_hash()));
                hashes
            },
        };
        merkle_root(&hashes)
    }

    pub fn is_final(&self, height: u32) -> bool {
        self.transactions.iter().all(|tx| tx.raw.is_final_in_block(height, self.header.raw.time))
    }
}

impl cmp::PartialEq for IndexedBlock {
    fn eq(&self, other: &Self) -> bool {
        self.header.hash == other.header.hash
    }
}

impl From<&'static str> for IndexedBlock {
    fn from(s: &'static str) -> Self {
        deserialize(&s.from_hex().unwrap() as &[u8]).unwrap()
    }
}
