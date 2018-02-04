use params::Magic;
use params::MAGIC_MAINNET;

#[derive(Debug, PartialEq, Clone)]
pub struct NetworkInfo {
	pub version: u32,
	pub magic: Magic,
}

pub const NETWORK_INFO: NetworkInfo = NetworkInfo
{
    version: 1,
	magic: MAGIC_MAINNET,
};