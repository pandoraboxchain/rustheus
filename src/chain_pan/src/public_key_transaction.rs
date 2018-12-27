
use hash::H256;
use crypto::dhash256;
use ser::{serialize, serialize_with_flags, SERIALIZE_TRANSACTION_WITNESS};
use ser::{Error, Serializable, Deserializable, Stream, Reader};
use std::io;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct PublicKeyTransaction {
    pub version: i32,
    pub pubkey_index: H256,
    pub random: u8
}

impl PublicKeyTransaction {
    pub fn hash(&self) -> H256 {
        dhash256(&serialize(self))
    }

    pub fn witness_hash(&self) -> H256 {
        dhash256(&serialize_with_flags(self, SERIALIZE_TRANSACTION_WITNESS))
    }
}

impl Serializable for PublicKeyTransaction {
    fn serialize(&self, s: &mut Stream) {
        unimplemented!()
    }
}

impl Deserializable for PublicKeyTransaction {
    fn deserialize<T>(reader: &mut Reader<T>) -> Result<Self, Error> where Self: Sized, T: io::Read {
        unimplemented!()
    }
}