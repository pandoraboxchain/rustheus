use chain::Transaction;
use std::sync::mpsc::{self, Sender, Receiver};

/// Wrapper around `Vec<Transaction>`
pub struct Mempool
{
    sender: Sender<Transaction>,
    receiver: Receiver<Transaction>,
    pub transactions: Vec<Transaction>
}

impl Mempool
{
    pub fn new() -> Self
    {
        let (sender, receiver) = mpsc::channel();
        let mempool = Mempool { sender, receiver, transactions: Vec::new() };
        mempool
    }

    pub fn get_sender(&self) -> Sender<Transaction>
    {
        self.sender.clone()
    }

    fn run(&mut self)
    {
        loop
        {
            if let Ok(transaction) = self.receiver.recv()
            {
                self.transactions.push(transaction);
            }
        }
    }
}