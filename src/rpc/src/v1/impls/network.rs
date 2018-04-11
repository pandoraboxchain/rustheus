use std::sync::Arc;
use std::net::{SocketAddr, IpAddr};
use v1::traits::Network as NetworkRpc;
use v1::types::{AddNodeOperation, NodeInfo};
use jsonrpc_core::Error;
use jsonrpc_macros::Trailing;
use v1::helpers::errors;

pub trait NetworkApi : Send + Sync + 'static {
	fn add_node(&self, socket_addr: SocketAddr) -> Result<(), Error>;
	fn remove_node(&self, socket_addr: SocketAddr) -> Result<(), Error>;
	fn connect(&self, socket_addr: SocketAddr);
	fn node_info(&self, node_addr: IpAddr) -> Result<NodeInfo, Error>;
	fn nodes_info(&self) -> Vec<NodeInfo>;
	fn connection_count(&self) -> usize;
}

impl<T> NetworkRpc for NetworkClient<T> where T: NetworkApi {
	fn add_node(&self, node: String, operation: AddNodeOperation) -> Result<(), Error> {
		unimplemented!();
	}

	fn node_info(&self, _dns: bool, node_addr: Trailing<String>) -> Result<Vec<NodeInfo>, Error> {
		unimplemented!();
	}

	fn connection_count(&self) -> Result<usize, Error> {
		unimplemented!();
	}
}

pub struct NetworkClient<T: NetworkApi> {
	api: T,
}

impl<T> NetworkClient<T> where T: NetworkApi {
	pub fn new(api: T) -> Self {
		NetworkClient {
			api: api,
		}
	}
}

pub struct NetworkClientCore {
}

impl NetworkClientCore {
	pub fn new() -> Self {
		NetworkClientCore { }
	}
}

impl NetworkApi for NetworkClientCore {
	fn add_node(&self, socket_addr: SocketAddr) -> Result<(), Error> {
		unimplemented!();
	}

	fn remove_node(&self, socket_addr: SocketAddr) -> Result<(), Error> {
		unimplemented!();
	}

	fn connect(&self, socket_addr: SocketAddr) {
		unimplemented!();
	}

	fn node_info(&self, node_addr: IpAddr) -> Result<NodeInfo, Error> {
		unimplemented!();
	}

	fn nodes_info(&self) -> Vec<NodeInfo> {
		unimplemented!();
	}

	fn connection_count(&self) -> usize {
		unimplemented!();
	}
}
