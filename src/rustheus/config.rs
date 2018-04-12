use clap;
use params::{NetworkParams, ConsensusParams, ConsensusFork};
use rpc_apis::ApiSet;
use rpc::HttpConfiguration as RpcHttpConfig;

#[derive(Clone)]
pub struct Config {
	pub is_first: bool,
	pub network: NetworkParams,
	pub consensus: ConsensusParams,
	pub number: u16,
	pub telnet_port: u16,
	pub rpc_config: RpcHttpConfig,

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

	let mut rpc_config = parse_rpc_config(network, matches)?;
	rpc_config.port += number;

	let config = Config {
		is_first,
		number,
		network,
		telnet_port,
		consensus,
		rpc_config
	};

	Ok(config)
}

fn parse_rpc_config(network: NetworkParams, matches: &clap::ArgMatches) -> Result<RpcHttpConfig, String> {
	let mut config = RpcHttpConfig::with_port(network.rpc_port());
	config.enabled = !matches.is_present("no-jsonrpc");
	if !config.enabled {
		return Ok(config);
	}

	if let Some(apis) = matches.value_of("jsonrpc-apis") {
		config.apis = ApiSet::List(vec![apis.parse().map_err(|_| "Invalid APIs".to_owned())?].into_iter().collect());
	}
	if let Some(port) = matches.value_of("jsonrpc-port") {
		config.port = port.parse().map_err(|_| "Invalid JSON RPC port".to_owned())?;
	}
	if let Some(interface) = matches.value_of("jsonrpc-interface") {
		config.interface = interface.to_owned();
	}
	if let Some(cors) = matches.value_of("jsonrpc-cors") {
		config.cors = Some(vec![cors.parse().map_err(|_| "Invalid JSON RPC CORS".to_owned())?]);
	}
	if let Some(hosts) = matches.value_of("jsonrpc-hosts") {
		config.hosts = Some(vec![hosts.parse().map_err(|_| "Invalid JSON RPC hosts".to_owned())?]);
	}

	Ok(config)
}
