/// -----------------------------------------------------
/// Payment btc transaction
/// -----------------------------------------------------

use std::io;
use heapsize::HeapSizeOf;
use hex::FromHex;
use ser::{deserialize, serialize, serialize_with_flags, SERIALIZE_TRANSACTION_WITNESS};
use crypto::dhash256;
use hash::H256;
use constants::{LOCKTIME_THRESHOLD};
use ser::{Error, Serializable, Deserializable, Stream, Reader};
use transaction_helper::{TransactionInput, TransactionOutput};

/// Must be zero.
const WITNESS_MARKER: u8 = 0;
/// Must be nonzero.
const WITNESS_FLAG: u8 = 1;

#[derive(Debug, PartialEq, Default, Clone)]
pub struct PaymentTransaction {
    pub version: i32,
    pub inputs: Vec<TransactionInput>,
    pub outputs: Vec<TransactionOutput>,
    pub lock_time: u32,
}

impl PaymentTransaction {
    pub fn hash(&self) -> H256 {
        dhash256(&serialize(self))
    }

    pub fn witness_hash(&self) -> H256 {
        dhash256(&serialize_with_flags(self, SERIALIZE_TRANSACTION_WITNESS))
    }

    pub fn has_witness(&self) -> bool {
        self.inputs.iter().any(TransactionInput::has_witness)
    }

    pub fn inputs(&self) -> &[TransactionInput] {
        &self.inputs
    }

    pub fn outputs(&self) -> &[TransactionOutput] {
        &self.outputs
    }

    pub fn is_empty(&self) -> bool {
        self.inputs.is_empty() || self.outputs.is_empty()
    }

    pub fn is_null(&self) -> bool {
        self.inputs.iter().any(|input| input.previous_output.is_null())
    }

    pub fn is_coinbase(&self) -> bool {
        self.inputs.len() == 1 && self.inputs[0].previous_output.is_null()
    }

    pub fn is_final(&self) -> bool {
        // if lock_time is 0, transaction is final
        if self.lock_time == 0 {
            return true;
        }
        // setting all sequence numbers to 0xffffffff disables the time lock, so if you want to use locktime,
        // at least one input must have a sequence number below the maximum.
        self.inputs.iter().all(TransactionInput::is_final)
    }

    pub fn is_final_in_block(&self, block_height: u32, block_time: u32) -> bool {
        if self.lock_time == 0 {
            return true;
        }

        let max_lock_time = if self.lock_time < LOCKTIME_THRESHOLD {
            block_height
        } else {
            block_time
        };

        if self.lock_time < max_lock_time {
            return true;
        }

        self.inputs.iter().all(TransactionInput::is_final)
    }

    pub fn total_spends(&self) -> u64 {
        let mut result = 0u64;
        for output in self.outputs.iter() {
            if u64::max_value() - result < output.value {
                return u64::max_value();
            }
            result += output.value;
        }
        result
    }
}

impl From<&'static str> for PaymentTransaction {
    fn from(s: &'static str) -> Self {
        deserialize(&s.from_hex().unwrap() as &[u8]).unwrap()
    }
}

impl HeapSizeOf for PaymentTransaction {
    fn heap_size_of_children(&self) -> usize {
        self.inputs.heap_size_of_children() + self.outputs.heap_size_of_children()
    }
}

impl Serializable for PaymentTransaction {
    fn serialize(&self, stream: &mut Stream) {
        let include_transaction_witness = stream.include_transaction_witness() && self.has_witness();
        match include_transaction_witness {
            false => stream
                .append(&self.version)
                .append_list(&self.inputs)
                .append_list(&self.outputs)
                .append(&self.lock_time),
            true => {
                stream
                    .append(&self.version)
                    .append(&WITNESS_MARKER)
                    .append(&WITNESS_FLAG)
                    .append_list(&self.inputs)
                    .append_list(&self.outputs);
                for input in &self.inputs {
                    stream.append_list(&input.script_witness);
                }
                stream.append(&self.lock_time)
            }
        };
    }
}

impl Deserializable for PaymentTransaction {
    fn deserialize<T>(reader: &mut Reader<T>) -> Result<Self, Error> where Self: Sized, T: io::Read {
        let version = reader.read()?;
        let mut inputs: Vec<TransactionInput> = reader.read_list()?;
        let read_witness = if inputs.is_empty() {
            let witness_flag: u8 = reader.read()?;
            if witness_flag != WITNESS_FLAG {
                return Err(Error::MalformedData);
            }

            inputs = reader.read_list()?;
            true
        } else {
            false
        };
        let outputs = reader.read_list()?;
        if read_witness {
            for input in inputs.iter_mut() {
                input.script_witness = reader.read_list()?;
            }
        }

        Ok(PaymentTransaction {
            version: version,
            inputs: inputs,
            outputs: outputs,
            lock_time: reader.read()?,
        })
    }
}