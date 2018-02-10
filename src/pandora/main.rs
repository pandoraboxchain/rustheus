#![deny(unused_must_use)]   //this deny is needed primarily not to forget to unwrap Sender::send()

extern crate routing;
extern crate clap;
extern crate lru_time_cache;
extern crate maidsafe_utilities;
extern crate env_logger;
extern crate shrust;
extern crate bitcrypto as crypto;
extern crate chain;
extern crate serialization as ser;
extern crate message;
extern crate params;
extern crate primitives;
extern crate db;

#[macro_use] extern crate log;
#[macro_use] extern crate unwrap;
#[macro_use] extern crate serde_derive;

use clap::*;

use std::thread;
use std::path::PathBuf;
use std::sync::Arc;

mod mempool; use mempool::Mempool;
mod network; use network::NetworkNode;
mod executor; use executor::Executor;
mod input_listener; use input_listener::InputListener;
mod message_wrapper; use message_wrapper::MessageWrapper;
mod message_handler; use message_handler::MessageHandler;
mod executor_tasks;
mod service; use service::Service;


fn main() {
    env_logger::init().unwrap();
    let matches = App::new("pandora")
        .about(
            "The crust peer will run, using any config file it can find to \
                try and bootstrap off any provided peers.",
        )
        .arg(
            Arg::with_name("first")
                .short("f")
                .long("first")
                .help(
                    "Keep sending random data at a maximum speed of RATE bytes/second to the \
                   first connected peer.",
                )
        )
        .get_matches();

    let is_first_node = matches.is_present("first");
    
    let mempool = Mempool::new();
    let path = PathBuf::from(".");
    let default_db_cache = 512;
    let store = Arc::new(db::BlockChainDatabase::open_at_path(path, default_db_cache).expect("Failed to open database"));
    // let store = db_utils::open_db(Some(".".to_owned()), default_db_cache)
    let mut message_handler = MessageHandler::new(mempool.get_sender(), store);    

    let mut network = NetworkNode::new(is_first_node, message_handler.get_sender());
    let network_sender = network.get_bytes_to_send_sender();

    let message_wrapper = MessageWrapper::new(network_sender);

    let mut executor = Executor::new(mempool, message_wrapper);
    let input_listener = InputListener::new(is_first_node, executor.get_sender());

    thread::spawn(move || executor.run() );
    thread::spawn(move || message_handler.run() );

    network.run();

    let _pandora = PandoraNode
        {
            network,
            //executor,
            input_listener
        };

    //let mut network = pandora.network;
}

pub struct PandoraNode
{
    network: NetworkNode,
    //executor: Executor,
    input_listener: InputListener
}
