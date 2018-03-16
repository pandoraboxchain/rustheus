#[macro_use]
extern crate lazy_static;

extern crate chain;
extern crate primitives;

mod params;
mod consensus;
pub mod info;

pub use primitives::{hash, compact};

pub use consensus::{ConsensusParams, ConsensusFork};
pub use params::{Magic, NetworkParams};
