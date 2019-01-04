/// -----------------------------------------------------
/// tx input and outputs for stake transfer
/// -----------------------------------------------------

use std::io;
use heapsize::HeapSizeOf;
use bytes::Bytes;
use hash::H256;
use ser::{Error, Serializable, Deserializable, Stream, Reader};
use constants::SEQUENCE_FINAL;

#[derive(Debug, PartialEq, Eq, Clone, Default, Serializable, Deserializable)]
pub struct OutPoint {
    pub hash: H256,
    pub index: u32,
}

impl OutPoint {
    pub fn null() -> Self {
        OutPoint {
            hash: H256::default(),
            index: u32::max_value(),
        }
    }

    pub fn is_null(&self) -> bool {
        self.hash.is_zero() && self.index == u32::max_value()
    }
}

#[derive(Debug, PartialEq, Default, Clone)]
pub struct TransactionInput {
    pub previous_output: OutPoint,
    pub script_sig: Bytes,
    pub sequence: u32,
    pub script_witness: Vec<Bytes>,
}

impl TransactionInput {
    pub fn coinbase(script_sig: Bytes) -> Self {
        TransactionInput {
            previous_output: OutPoint::null(),
            script_sig: script_sig,
            sequence: SEQUENCE_FINAL,
            script_witness: vec![],
        }
    }

    pub fn is_final(&self) -> bool {
        self.sequence == SEQUENCE_FINAL
    }

    pub fn has_witness(&self) -> bool {
        !self.script_witness.is_empty()
    }
}

impl HeapSizeOf for TransactionInput {
    fn heap_size_of_children(&self) -> usize {
        self.script_sig.heap_size_of_children() +
            self.script_witness.heap_size_of_children()
    }
}

impl Serializable for TransactionInput {
    fn serialize(&self, stream: &mut Stream) {
        stream
            .append(&self.previous_output)
            .append(&self.script_sig)
            .append(&self.sequence);
    }
}

impl Deserializable for TransactionInput {
    fn deserialize<T>(reader: &mut Reader<T>) -> Result<Self, Error> where Self: Sized, T: io::Read {
        Ok(TransactionInput {
            previous_output: reader.read()?,
            script_sig: reader.read()?,
            sequence: reader.read()?,
            script_witness: vec![],
        })
    }
}


#[derive(Debug, PartialEq, Clone, Serializable, Deserializable)]
pub struct TransactionOutput {
    pub value: u64,
    pub script_pubkey: Bytes,
}

impl Default for TransactionOutput {
    fn default() -> Self {
        TransactionOutput {
            value: 0xffffffffffffffffu64,
            script_pubkey: Bytes::default(),
        }
    }
}

impl HeapSizeOf for TransactionOutput {
    fn heap_size_of_children(&self) -> usize {
        self.script_pubkey.heap_size_of_children()
    }
}
