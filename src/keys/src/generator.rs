use rand::os::OsRng;
use rand::Rng;
use network::Network;
use {KeyPair, SECP256K1, Error};

pub trait Generator {
	fn generate(&self) -> Result<KeyPair, Error>;
}

pub struct Random {
	network: Network
}

impl Random {
	pub fn new(network: Network) -> Self {
		Random {
			network: network,
		}
	}

	pub fn generate_bytes(bytes: &mut [u8]) -> Result<(), Error>
	{
		let mut rng = try!(OsRng::new().map_err(|_| Error::FailedKeyGeneration));		
		rng.fill_bytes(bytes);
		Ok(())
	}
}

impl Generator for Random {
	fn generate(&self) -> Result<KeyPair, Error> {
		let context = &SECP256K1;
		let mut rng = try!(OsRng::new().map_err(|_| Error::FailedKeyGeneration));
		let (secret, public) = try!(context.generate_keypair(&mut rng));
		Ok(KeyPair::from_keypair_compressed(secret, public, self.network))
	}
}
