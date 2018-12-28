use hash::H256;
use hex::FromHex;
use crypto::dhash256;
use ser::{serialize, deserialize};
use ser::{Error, Serializable, Deserializable, Stream, Reader};
use std::io;
use keys::Private;

#[derive(Debug, PartialEq)]
pub struct PrivateKeyTransaction {
    pub version: i32,
    pub key: Private
}

impl PrivateKeyTransaction {
    pub fn hash(&self) -> H256 {
        dhash256(&serialize(self))
    }

    pub fn version(&self) -> &i32 {
        &self.version
    }

    pub fn key(&self) -> &Private {
        &self.key
    }
}

impl From<&'static str> for PrivateKeyTransaction {
    fn from(s: &'static str) -> Self {
        deserialize(&s.from_hex().unwrap() as &[u8]).unwrap()
    }
}

impl Serializable for PrivateKeyTransaction {
    fn serialize(&self, stream: &mut Stream) {
        stream
            .append(&self.version)
            .append(&self.key);
    }
}

impl Deserializable for PrivateKeyTransaction {
    fn deserialize<T>(reader: &mut Reader<T>) -> Result<Self, Error> where Self: Sized, T: io::Read {
        Ok(PrivateKeyTransaction {
            version : reader.read()?,
            key : reader.read()?
        })
    }
}

#[cfg(test)]
mod test {

    use super::PrivateKeyTransaction;
    use ser::Serializable;
    use keys::Private;
    use hash::H256;

    #[test]
    fn test_transaction_reader() {
        let actual : PrivateKeyTransaction = "0100000000d53b80842f4ea32806ce5e723a255ddd6490cfd28dac38c58bf9254c0577330600".into();
        let key: Private = "5KSCKP8NUyBZPCCQusxRwgmz9sfvJQEgbGukmmHepWw5Bzp95mu".into();
        let expected : PrivateKeyTransaction = PrivateKeyTransaction {
            version : 1,
            key
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_transaction_hash() {
        let tx: PrivateKeyTransaction = "0100000000d53b80842f4ea32806ce5e723a255ddd6490cfd28dac38c58bf9254c0577330600".into();
        let hash : H256 = H256::from_reversed_str("a7e2e4dfda49a37effa04565b7fec377f3cf37be4ac947bc2b23a4e07d77b3d0");
        assert_eq!(tx.hash(), hash);
    }

    #[test]
    fn test_transaction_serialized_len() {
        let raw_tx: &'static str = "0100000000d53b80842f4ea32806ce5e723a255ddd6490cfd28dac38c58bf9254c0577330600";
        let tx: PrivateKeyTransaction = raw_tx.into();
        assert_eq!(tx.serialized_size(), raw_tx.len() / 2);
    }

}