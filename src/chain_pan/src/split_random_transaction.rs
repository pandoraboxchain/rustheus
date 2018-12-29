use hash::H256;
use hex::FromHex;
use crypto::dhash256;
use ser::{serialize, deserialize};
use ser::{Error, Serializable, Deserializable, Stream, Reader};

#[derive(Debug, PartialEq, Clone, Default, Serializable, Deserializable)]
pub struct SplitRandomTransaction {
    pub version: i32,
    pub pubkey_index: u8,
    pub pieces: u32     //TODO change to proper type
}

impl SplitRandomTransaction {
    pub fn hash(&self) -> H256 {
        dhash256(&serialize(self))
    }

    pub fn pubkey_index(&self) -> &u8 {
        &self.pubkey_index
    }

    pub fn pieces(&self) -> &u32 {
        &self.pieces
    }
}

impl From<&'static str> for SplitRandomTransaction {
    fn from(s: &'static str) -> Self {
        deserialize(&s.from_hex().unwrap() as &[u8]).unwrap()
    }
}

//TODO with complex type of pieces implement Serializable and Deserializable traits proper

#[cfg(test)]
mod test {

    use super::SplitRandomTransaction;
    use hash::H256;
    use ser::Serializable;

    #[test]
    fn test_transaction_reader() {
        let actual: SplitRandomTransaction = "010000000203000000".into();
        let expected : SplitRandomTransaction = SplitRandomTransaction {
            version : 1,
            pubkey_index : 2,
            pieces : 3
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_transaction_hash() {
        let tx: SplitRandomTransaction = "010000000203000000".into();
        let hash : H256 = H256::from_reversed_str("6b281addb22f8aa53a67e9b5f460ac6096c1c7e8b5c5bc36e6fa589372ac670c");
        assert_eq!(tx.hash(), hash);
    }

    #[test]
    fn test_transaction_serialized_len() {
        let raw_tx: &'static str = "010000000203000000";
        let tx: SplitRandomTransaction = raw_tx.into();
        assert_eq!(tx.serialized_size(), raw_tx.len() / 2);
    }

}