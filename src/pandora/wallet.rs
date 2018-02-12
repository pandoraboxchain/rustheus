use keys::generator::{Random, Generator};
use keys::network::Network;
use keys::{KeyPair, Private, Error};

pub struct Wallet
{
    keys: KeyPair
}

impl Wallet
{
    pub fn new() -> Result<Self, Error>
    {
        let generator = Random::new(Network::Mainnet);
        match generator.generate()
        {
            Ok(keys) =>
            {
                info!("got keys {}", keys);
                info!("address is {}", keys.address());
                Ok(Wallet { keys })
            } 
            Err(error) => Err(error)
        }
    }

    pub fn from_private(private: Private) -> Result<Self, Error>
    {
        match KeyPair::from_private(private)
        {
            Ok(keys) =>
            {
                info!("got keys {}", keys);
                info!("address is {}", keys.address());
                Ok(Wallet { keys })
            } 
            Err(error) => Err(error)
        }
    }
}