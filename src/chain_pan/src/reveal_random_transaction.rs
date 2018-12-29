use hash::H256;
use hex::FromHex;
use crypto::dhash256;
use ser::{serialize, deserialize};
use ser::{Error, Serializable, Deserializable, Stream, Reader};
use keys::Private;

#[derive(Debug, PartialEq, Serializable, Deserializable)]
pub struct RevealRandomTransaction {
    pub version: i32,
    pub commit_hash: H256,
    pub key: Private
}

impl RevealRandomTransaction {
    pub fn hash(&self) -> H256 {
        dhash256(&serialize(self))
    }

    pub fn commit_hash(&self) -> &H256 {
        &self.commit_hash
    }

    pub fn key(&self) -> &Private {
        &self.key
    }
}

impl From<&'static str> for RevealRandomTransaction {
    fn from(s: &'static str) -> Self {
        deserialize(&s.from_hex().unwrap() as &[u8]).unwrap()
    }
}

#[cfg(test)]
mod test {

    use hash::H256;
    use ser::Serializable;
    use super::RevealRandomTransaction;
    use keys::Private;
    use std::str;

    #[test]
    fn test_transaction_reader() {
        let actual : RevealRandomTransaction = "01000000000000000000000000000000000000000000000000000000000000000000000000d53b80842f4ea32806ce5e723a255ddd6490cfd28dac38c58bf9254c0577330600".into();
        let key: Private = "5KSCKP8NUyBZPCCQusxRwgmz9sfvJQEgbGukmmHepWw5Bzp95mu".into();
        let expected : RevealRandomTransaction = RevealRandomTransaction {
            version: 1,
            commit_hash: H256::from(0),
            key
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_transaction_hash() {
        let tx: RevealRandomTransaction = "01000000000000000000000000000000000000000000000000000000000000000000000000d53b80842f4ea32806ce5e723a255ddd6490cfd28dac38c58bf9254c0577330600".into();
        let hash : H256 = H256::from_reversed_str("b7f4ebf401284a85bc2de5ca080aa1f816b8b6cdcf73adcf90fd93be37ae891b");
        assert_eq!(tx.hash(), hash);
    }

    #[test]
    fn test_transaction_serialized_len() {
        let raw_tx: &'static str = "01000000000000000000000000000000000000000000000000000000000000000000000000d53b80842f4ea32806ce5e723a255ddd6490cfd28dac38c58bf9254c0577330600";
        let tx: RevealRandomTransaction = raw_tx.into();
        assert_eq!(tx.serialized_size(), raw_tx.len() / 2);
    }
}
