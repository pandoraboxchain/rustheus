use keys::generator::{Random, Generator};
use keys::network::Network;
use wallet_manager_tasks::Task;
use std::sync::mpsc::{self, Sender, Receiver};
use service::Service;

pub struct WalletManager
{
    receiver: Receiver<Task>,
    sender: Sender<Task>,
}

impl WalletManager
{
    pub fn new() -> Self
    {
        let (sender, receiver) = mpsc::channel();
        WalletManager
        {
            sender,
            receiver
        }
    }

    fn create_wallet(&self)
    {
        let generator = Random::new(Network::Mainnet);
        match generator.generate()
        {
            Ok(keypair) =>
            {
                info!("got keypair {}", keypair);
                info!("address is {}", keypair.address());
            } 
            Err(error) => error!("error generating keypair {:?}", error)
        }
    }
}

impl Service for WalletManager
{
    type Item = Task;
    fn get_sender(&self) -> Sender<Self::Item>
    {
        self.sender.clone()
    }

    fn run(&mut self)
    {
        loop
        {
            if let Ok(task) = self.receiver.recv()
            {
                info!("wallet task received, it is {:?}", task);
                match task
                {
                    Task::CreateWallet() => self.create_wallet(),
                    Task::SendCash(value) => {}
                }
            }
        } 
    }
}

