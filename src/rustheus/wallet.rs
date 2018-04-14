use keys::generator::{Random, Generator};
use keys::network::Network;
use keys::{KeyPair, Private, Error, Address};

pub struct Wallet
{
    pub keys: Vec<KeyPair>
}

impl Wallet
{
    pub fn new() -> Self
    {
       Wallet { keys: vec![] }
    }

    pub fn new_keypair(&mut self) -> Address {
        //TODO testnet support
        let generator = Random::new(Network::Mainnet);
        let keypair = generator.generate().expect("Could not generate keypair");
        let address = keypair.address();
        info!("Generated keypair {}", keypair);
        info!("Address is {}", address);
        self.keys.push(keypair);
        address
    }

    pub fn add_keypair_from_private(&mut self, private: Private) -> Result<Address, Error>
    {
        match KeyPair::from_private(private)
        {
            Ok(keypair) =>
            {
                let address = keypair.address();
                info!("Added keys {}", keypair);
                info!("Public key hash is {}", address.hash);
                info!("Address is {}", address);
                self.keys.push(keypair);
                Ok(address)
            } 
            Err(error) => Err(error)
        }
    }
}