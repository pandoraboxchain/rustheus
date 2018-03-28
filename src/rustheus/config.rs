use clap;
use params::{NetworkParams, ConsensusParams, ConsensusFork};

#[derive(Clone)]
pub struct Config {
	pub is_first: bool,
	pub network: NetworkParams,
	pub consensus: ConsensusParams,
	pub number: u16,
	pub telnet_port: u16,
}

pub const DEFAULT_DB_CACHE: usize = 512;
pub const DEFAULT_TELNET_PORT: u16 = 4070;

pub fn parse(matches: &clap::ArgMatches) -> Result<Config, String> {

	let network = match matches.is_present("testnet") {
		true => NetworkParams::Testnet,
		false => NetworkParams::Mainnet,
	};

	let consensus = ConsensusParams::new(network, ConsensusFork::NoFork);

	let number = matches
        .value_of("number")
        .unwrap_or("0")
        .parse::<u16>()
        .expect("Node number is incorrect");

	let telnet_port = DEFAULT_TELNET_PORT + number;

    let is_first = matches.is_present("first");	

	let config = Config {
		is_first,
		number,
		network,
		telnet_port,
		consensus
	};

	Ok(config)
}
