use hash::H256;
use hex::FromHex;
use crypto::dhash256;
use ser::{serialize, deserialize};
use ser::{Error, Serializable, Deserializable, Stream, Reader};
use std::io;
use keys::Private;

#[derive(Debug, PartialEq)]
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

impl Serializable for RevealRandomTransaction {
    fn serialize(&self, stream: &mut Stream) {
        stream.append(&self.version)
            .append(&self.commit_hash)
            .append(&self.key);
    }
}

impl Deserializable for RevealRandomTransaction {
    fn deserialize<T>(reader: &mut Reader<T>) -> Result<Self, Error> where Self: Sized, T: io::Read {
        let test_version: i32 = reader.read()?;
        let test_commit_hash: H256 = reader.read()?;
        let test_key: Private = reader.read()?; // cant read private crashes on malformed data
        Ok(RevealRandomTransaction {
            version: test_version,
            commit_hash : test_commit_hash,
            key : test_key
        })
    }
}

#[cfg(test)]
mod test {

    use hash::H256;
    use ser::{Serializable, serialize, deserialize};
    use super::RevealRandomTransaction;
    use keys::Private;
    use hex::FromHex;
    use std::str;

    #[test]
    fn test_transaction_reader() {
        // default Private serialization methods
        let key: Private = "5KSCKP8NUyBZPCCQusxRwgmz9sfvJQEgbGukmmHepWw5Bzp95mu".into();
//        let mut key_serialized_by_to_string = key.to_string();
//        assert_eq!("5KSCKP8NUyBZPCCQusxRwgmz9sfvJQEgbGukmmHepWw5Bzp95mu", key_serialized_by_to_string);

        // test new serialization
//        let mut key_serialized = serialize(&key); // by serializator
//        let mut hex_test = "00000000d53b80842f4ea32806ce5e723a255ddd6490cfd28dac38c58bf9254c0577330600".from_hex().unwrap();
        //let mut t : Private = hex_test.into();
//        println!("{:?}", key_serialized);
//        println!("{:?}", key_serialized_by_to_string);
        
        let ser_key = serialize(&key);
        println!("key {:?}", ser_key);
        let deser_key: Private = deserialize(&ser_key as &[u8]).unwrap();

        let mut string_key = key.to_string();
        let mut test : RevealRandomTransaction = RevealRandomTransaction {
            version: 1,
            commit_hash: H256::from(0),
            key
        };
        let mut serializedTx = serialize(&test);
        println!("{:?}", serializedTx);
        
        let mut deserializedTx : RevealRandomTransaction = "01000000000000000000000000000000000000000000000000000000000000000000000000d53b80842f4ea32806ce5e723a255ddd6490cfd28dac38c58bf9254c0577330600".into();
        println!("{}", "test");
    }

    #[test]
    fn test_transaction_hash() {

    }

    #[test]
    fn test_transaction_serialized_len() {

    }
}
