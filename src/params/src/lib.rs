#[macro_use]
extern crate lazy_static;

extern crate chain_pan as chain;
extern crate primitives;

mod params;
mod consensus;

pub use primitives::{hash, compact};

pub use consensus::{ConsensusParams, ConsensusFork};
pub use params::{Magic, NetworkParams};
