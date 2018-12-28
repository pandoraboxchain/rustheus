use hash::H256;
use hex::FromHex;
use crypto::dhash256;
use ser::{serialize, deserialize};
use ser::{Error, Serializable, Deserializable, Stream, Reader};
use std::io;
use keys::Public;

#[derive(Debug, PartialEq)]
pub struct PublicKeyTransaction {
    pub version: i32,
    pub generated_pubkey: Public,
    pub pubkey_index: u8
}

impl PublicKeyTransaction {
    pub fn hash(&self) -> H256 {
        dhash256(&serialize(self))
    }

    pub fn generated_pubkey(&self) -> &Public {
        &self.generated_pubkey
    }

    pub fn pubkey_index(&self) -> &u8 {
        &self.pubkey_index
    }
}

impl From<&'static str> for PublicKeyTransaction {
    fn from(s: &'static str) -> Self {
        deserialize(&s.from_hex().unwrap() as &[u8]).unwrap()
    }
}

impl Serializable for PublicKeyTransaction {
    fn serialize(&self, stream: &mut Stream) {
        stream
            .append(&self.version)
            .append(&self.generated_pubkey)
            .append(&self.pubkey_index);
    }
}

impl Deserializable for PublicKeyTransaction {
    fn deserialize<T>(reader: &mut Reader<T>) -> Result<Self, Error> where Self: Sized, T: io::Read {
        Ok(PublicKeyTransaction {
            version : reader.read()?,
            generated_pubkey : reader.read()?,
            pubkey_index : reader.read()?
        })
    }
}

#[cfg(test)]
mod test {

    use keys::Public;
    use hash::H264;
    use hash::H256;
    use super::PublicKeyTransaction;
    use ser::Serializable;

    #[test]
    fn test_transaction_reader() {
        let actual : PublicKeyTransaction = "010000000100000000000000000000000000000000000000000000000000000000000000000002".into();
        let generated_pubkey : Public = Public::Compressed(H264::from(0));
        let expected : PublicKeyTransaction = PublicKeyTransaction {
            version : 1,
            generated_pubkey,
            pubkey_index : 2
        };
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_transaction_hash() {
        let tx: PublicKeyTransaction = "010000000100000000000000000000000000000000000000000000000000000000000000000002".into();
        let hash : H256 = H256::from_reversed_str("fe95829a0b14713665d3e53e385388452504e57cc561f31cbaf10ddbc78a4437");
        assert_eq!(tx.hash(), hash);
    }

    #[test]
    fn test_transaction_serialized_len() {
        let raw_tx: &'static str = "010000000100000000000000000000000000000000000000000000000000000000000000000002";
        let tx: PublicKeyTransaction = raw_tx.into();
        assert_eq!(tx.serialized_size(), raw_tx.len() / 2);
    }

}