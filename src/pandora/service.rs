use std::sync::mpsc::Sender;

pub trait Service
{
    type Item;
    fn get_sender(&self) -> Sender<Self::Item>;
    fn run(&mut self);
}