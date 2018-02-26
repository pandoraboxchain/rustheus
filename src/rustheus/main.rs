#![deny(unused_must_use)]   //this deny is needed primarily not to forget to unwrap Sender::send()

extern crate routing;
extern crate clap;
extern crate lru_time_cache;
extern crate maidsafe_utilities;
extern crate pretty_env_logger;
extern crate shrust;
extern crate bitcrypto as crypto;
extern crate chain;
extern crate serialization as ser;
extern crate message;
extern crate params;
extern crate primitives;
extern crate db;
extern crate keys;
extern crate script;
extern crate ctrlc;

#[macro_use] extern crate log;
#[macro_use] extern crate unwrap;
#[macro_use] extern crate serde_derive;

use clap::*;

use std::thread;
use params::NetworkParams;
use std::sync::{Arc, RwLock};
use std::process;
use std::sync::mpsc;

mod mempool; use mempool::Mempool;
mod network; use network::NetworkNode;
mod executor; use executor::Executor;
mod input_listener; use input_listener::InputListener;
mod message_wrapper; use message_wrapper::MessageWrapper;
mod message_handler; use message_handler::MessageHandler;
mod executor_tasks;
mod service; use service::Service;
mod db_utils;
mod wallet_manager; mod wallet_manager_tasks; use wallet_manager::WalletManager;
mod wallet;
mod responder; use responder::Responder;


fn main() {
    pretty_env_logger::init();
    let matches = App::new("pandora")
        .about(
            "The crust peer will run, using any config file it can find to \
                try and bootstrap off any provided peers.",
        )
        .arg(
            Arg::with_name("first")
                .short("f")
                .long("first")
                .help("Indicates if this node be bootstraping node")
        )
        .arg(
            Arg::with_name("number")
                .short("n")
                .long("number")
                .help("Number for node unique database for debug usage. Allows have databases inside different folders and unique telnet ports to communicate")
                .takes_value(true)
        )
        .get_matches();

    let is_first_node = matches.is_present("first");

    //setup database
    let db_path_string = "./db".to_owned() + matches.value_of("number").unwrap_or("") + "/";
    let default_db_cache = 512;
    let storage = db_utils::open_db(db_path_string, default_db_cache);
    db_utils::init_db(storage.clone(), NetworkParams::Mainnet).unwrap(); //init db with genesis block
    
    //setup mempool
    let mempool_ref = Arc::new(RwLock::new(Mempool::new()));

    //setup cross thread communication channels
    let (to_network_sender, to_network_receiver) = mpsc::channel();
    let (from_network_sender, from_network_receiver) = mpsc::channel();
    let (responder_task_sender, responder_task_receiver) = mpsc::channel();
    let (terminate_sender, terminate_receiver) = mpsc::channel();

    //setup network requests responder
    let mut responder = Responder {
        storage: storage.clone(),
        task_receiver: responder_task_receiver,
        message_wrapper: MessageWrapper::new(to_network_sender.clone())
    };
    //setup network messages handler
    let mut message_handler = MessageHandler {
        mempool: mempool_ref.clone(),
        store: storage.clone(),
        network_data_receiver: from_network_receiver,
        network_responder: responder_task_sender.clone()
    };

    //setup p2p layer
    let mut network = NetworkNode::new(
        is_first_node,
        from_network_sender,
        to_network_receiver,
        terminate_receiver,
    );

    //setup wallet task and miscellaneous task executor
    let (mut wallet_manager, wallet_manager_sender) = WalletManager::new(mempool_ref.clone(), storage.clone(), MessageWrapper::new(to_network_sender.clone()));
    let (mut executor, executor_sender) = Executor::new(mempool_ref.clone(), storage.clone(), MessageWrapper::new(to_network_sender.clone()));
    
    //setup telnet listener
    let node_unique_number = matches.value_of("number").unwrap_or("0").parse::<u32>().expect("Node number is incorrect");
    let input_listener = InputListener::new(node_unique_number, executor_sender, wallet_manager_sender, terminate_sender);

    //launch services in different threads
    let input_listener_thread = thread::spawn( move || input_listener.run() );
    let responder_thread = thread::spawn( move || responder.run() );
    let executor_thread = thread::spawn( move || executor.run() );
    let wallet_manager_thread = thread::spawn( move || wallet_manager.run() );    
    let message_handler_thread = thread::spawn( move || message_handler.run() );

    //prepare to handle Ctrl-C
    ctrlc::set_handler(move || {
        info!("Interrupted. Your blockchain latest state may not be saved. Please use `shutdown` command to exit properly");
        process::exit(0);
        //TODO send interrupt to input_listener and network_node, so we can exit properly even without `shutdown` command
        //interrupt_sender.send(true).expect("Could not exit properly. Blockchain latest state may be not saved");
    }).expect("Error setting Ctrl-C handler");

    network.run();  //main thread loop
    drop(network);  //remove everything after network loop was finished

    //wait for other threads to finish
    responder_thread.join().unwrap();
    executor_thread.join().unwrap();   
    wallet_manager_thread.join().unwrap();
    input_listener_thread.join().unwrap();
    message_handler_thread.join().unwrap();
}