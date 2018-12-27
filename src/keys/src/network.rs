use std::io;
use ser::{Serializable, Deserializable, Stream, Reader, Error};

#[derive(Debug, PartialEq, Clone, Copy)]
pub enum Network {
	Mainnet,
	Testnet,
}

impl Serializable for Network {
	fn serialize(&self, stream: &mut Stream) {
		match self {
			&Network::Mainnet => {stream.append(&0);},
			&Network::Testnet => {stream.append(&1);}
		}
	}
}

impl Deserializable for Network {
	fn deserialize<T>(reader: &mut Reader<T>) -> Result<Self, Error> where Self: Sized, T: io::Read {
		let value: u8 = reader.read()?;
		match value {
			0 => Ok(Network::Mainnet),
			1 => Ok(Network::Testnet),
			_ => Ok(Network::Testnet)
		}
	}
}