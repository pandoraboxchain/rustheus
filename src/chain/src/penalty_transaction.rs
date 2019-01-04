use hash::H256;
use hex::FromHex;
use crypto::dhash256;
use ser::{serialize, deserialize};
use ser::{Error, Serializable, Deserializable, Stream, Reader};
use std::io;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct PenaltyTransaction {
    pub version: i32,
    pub conflicts: Vec<H256>,
}

impl PenaltyTransaction {
    pub fn hash(&self) -> H256 {
        dhash256(&serialize(self))
    }

    pub fn conflicts(&self) -> &Vec<H256> {
        &self.conflicts
    }
}

impl From<&'static str> for PenaltyTransaction {
    fn from(s: &'static str) -> Self {
        deserialize(&s.from_hex().unwrap() as &[u8]).unwrap()
    }
}

impl Serializable for PenaltyTransaction {
    fn serialize(&self, stream: &mut Stream) {
        stream.append(&self.version)
            .append_list(&self.conflicts);
    }
}

impl Deserializable for PenaltyTransaction {
    fn deserialize<T>(reader: &mut Reader<T>) -> Result<Self, Error> where Self: Sized, T: io::Read {
        let version = reader.read()?;
        let conflicts: Vec<H256> = reader.read_list()?;
        Ok(PenaltyTransaction {
            version,
            conflicts
        })
    }
}

#[cfg(test)]
mod tests {

    use hash::H256;
    use ser::Serializable;
    use super::PenaltyTransaction;

    #[test]
    fn test_transaction_reader() {
        let actual: PenaltyTransaction = "010000000200000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000".into();
        let expected: PenaltyTransaction = PenaltyTransaction {
            version:1,
            conflicts: vec![H256::from(0), H256::from(1)]
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_transaction_hash() {
        let tx: PenaltyTransaction = "010000000200000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000".into();
        let hash : H256 = H256::from_reversed_str("0d21688912fcd0ff33eb55731f38ab9efd4c4704c00ec3acd88b00b1d8afed28");
        assert_eq!(tx.hash(), hash);
    }

    #[test]
    fn test_transaction_serialized_len() {
        let raw_tx: &'static str = "010000000200000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000";
        let tx: PenaltyTransaction = raw_tx.into();
        assert_eq!(tx.serialized_size(), raw_tx.len() / 2);
    }
}