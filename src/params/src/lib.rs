#[macro_use]
extern crate lazy_static;

extern crate chain;
extern crate primitives;

mod params;
pub mod info;

pub use primitives::{hash, compact};

pub use params::{Magic, NetworkParams};
