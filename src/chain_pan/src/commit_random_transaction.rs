use hash::H256;
use hex::FromHex;
use crypto::dhash256;
use ser::{serialize, deserialize};
use ser::{Error, Serializable, Deserializable, Stream, Reader};
use std::io;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct CommitRandomTransaction {
    pub version: i32,
    pub random: Vec<u32>,    // 75 bytes
    pub pubkey_index: u16    // sets only two bytes pubkey index
}

impl CommitRandomTransaction {
    pub fn hash(&self) -> H256 {
        dhash256(&serialize(self))
    }

    pub fn random(&self) -> &Vec<u32> {
        &self.random
    }

    pub fn pubkey_index(&self) -> &u16 {
        &self.pubkey_index
    }
}

impl From<&'static str> for CommitRandomTransaction {
    fn from(s: &'static str) -> Self {
        deserialize(&s.from_hex().unwrap() as &[u8]).unwrap()
    }
}

impl Serializable for CommitRandomTransaction {
    fn serialize(&self, stream: &mut Stream) {
        stream.append(&self.version)
            .append_list(&self.random)
            .append(&self.pubkey_index);
    }
}

impl Deserializable for CommitRandomTransaction {
    fn deserialize<T>(reader: &mut Reader<T>) -> Result<Self, Error> where Self: Sized, T: io::Read {
        let version: i32 = reader.read()?;
        let random: Vec<u32> = reader.read_list()?;
        let pubkey_index: u16 = reader.read()?;
        Ok( CommitRandomTransaction {
            version,
            random,
            pubkey_index
        })

    }
}

#[cfg(test)]
mod test {

    use hash::H256;
    use ser::{Serializable, serialize};
    use super::CommitRandomTransaction;

    #[test]
    fn test_transaction_reader() {
        let actual : CommitRandomTransaction = "0100000002010000004b0000000100".into();
        let expected: CommitRandomTransaction = CommitRandomTransaction {
            version:1,
            random: vec![1, 75],
            pubkey_index:1
        };
        let mut serialized = serialize(&expected);
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_transaction_hash() {
        let tx: CommitRandomTransaction = "0100000002010000004b0000000100".into();
        let hash : H256 = H256::from_reversed_str("3b848c607d9114fc2c010742349fd629c1323a44ad28516bfd25f77f21920657");
        assert_eq!(tx.hash(), hash);
    }

    #[test]
    fn test_transaction_serialized_len(){
        let raw_tx: &'static str = "0100000002010000004b0000000100";
        let tx: CommitRandomTransaction = raw_tx.into();
        assert_eq!(tx.serialized_size(), raw_tx.len() / 2);
    }
}