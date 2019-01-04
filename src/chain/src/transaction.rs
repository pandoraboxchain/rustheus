use hash::H256;
use std::io;
use payment_transaction::PaymentTransaction;
use penalty_transaction::PenaltyTransaction;
use commit_random_transaction::CommitRandomTransaction;
use reveal_random_transaction::RevealRandomTransaction;
use public_key_transaction::PublicKeyTransaction;
use private_key_transaction::PrivateKeyTransaction;
use split_random_transaction::SplitRandomTransaction;
use ser::{Serializable, Deserializable, Error, Stream, Reader};

#[derive(Debug, PartialEq)]
pub enum Transaction {
    PaymentTransaction(PaymentTransaction),
    PenaltyTransaction(PenaltyTransaction),
    CommitRandomTransaction(CommitRandomTransaction),
    RevealRandomTransaction(RevealRandomTransaction),
    PublicKeyTransaction(PublicKeyTransaction),
    PrivateKeyTransaction(PrivateKeyTransaction),
    SplitRandomTransaction(SplitRandomTransaction)
}

impl Transaction {
     pub fn hash(&self) -> H256 {
        match self {
            &Transaction::PaymentTransaction(ref _tx) => _tx.hash(),
            &Transaction::PenaltyTransaction(ref _tx) => _tx.hash(),
            &Transaction::CommitRandomTransaction(ref _tx) => _tx.hash(),
            &Transaction::RevealRandomTransaction(ref _tx) => _tx.hash(),
            &Transaction::PublicKeyTransaction(ref _tx) => _tx.hash(),
            &Transaction::PrivateKeyTransaction(ref _tx) => _tx.hash(),
            &Transaction::SplitRandomTransaction(ref _tx) => _tx.hash()
        }
    }

    pub fn witness_hash(&self) -> H256 {
        match self {
            &Transaction::PaymentTransaction(ref _tx) => _tx.hash(),
            &Transaction::PenaltyTransaction(ref _tx) => _tx.hash(),
            &Transaction::CommitRandomTransaction(ref _tx) => _tx.hash(),
            &Transaction::RevealRandomTransaction(ref _tx) => _tx.hash(),
            &Transaction::PublicKeyTransaction(ref _tx) => _tx.hash(),
            &Transaction::PrivateKeyTransaction(ref _tx) => _tx.hash(),
            &Transaction::SplitRandomTransaction(ref _tx) => _tx.hash()
        }
    }

    pub fn has_witness(&self) -> bool {
        match self {
            &Transaction::PaymentTransaction(ref _tx) => true,
            &Transaction::PenaltyTransaction(ref _tx) => true,
            &Transaction::CommitRandomTransaction(ref _tx) => true,
            &Transaction::RevealRandomTransaction(ref _tx) => true,
            &Transaction::PublicKeyTransaction(ref _tx) => true,
            &Transaction::PrivateKeyTransaction(ref _tx) => true,
            &Transaction::SplitRandomTransaction(ref _tx) => true
        }
    }

    pub fn is_final_in_block(&self, block_height: u32, block_time: u32) -> bool {
        match self {
            &Transaction::PaymentTransaction(ref _tx) => { true },
            &Transaction::PenaltyTransaction(ref _tx) => { true },
            &Transaction::CommitRandomTransaction(ref _tx) => { true },
            &Transaction::RevealRandomTransaction(ref _tx) => { true },
            &Transaction::PublicKeyTransaction(ref _tx) => { true },
            &Transaction::PrivateKeyTransaction(ref _tx) => { true },
            &Transaction::SplitRandomTransaction(ref _tx) => { true }
        }
    }
}

impl Serializable for Transaction {
    fn serialize(&self, stream: &mut Stream) {
        unimplemented!()
    }
}

impl Deserializable for Transaction {
    fn deserialize<T>(reader: &mut Reader<T>) -> Result<Self, Error> where Self: Sized, T: io::Read {
        unimplemented!()
    }
}

impl Default for Transaction {
    fn default() -> Self {
        unimplemented!()
    }
}

impl Clone for Transaction {
    fn clone(&self) -> Self {
        unimplemented!()
    }
}
