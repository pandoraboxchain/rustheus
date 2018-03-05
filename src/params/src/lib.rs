#[macro_use]
extern crate lazy_static;

extern crate chain;
extern crate primitives;

mod params;
mod consensus;
mod deployments;
pub mod info;

pub use primitives::{hash, compact};

pub use consensus::{ConsensusParams, ConsensusFork};
pub use deployments::Deployment;
pub use params::{Magic, NetworkParams};
