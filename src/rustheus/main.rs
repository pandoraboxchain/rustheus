#![deny(unused_must_use)] //this deny is needed primarily not to forget to unwrap Sender::send()

extern crate bitcrypto as crypto;
extern crate chain;
extern crate clap;
extern crate ctrlc;
extern crate db;
extern crate keys;

extern crate futures_cpupool;
extern crate memory_pool;
extern crate message;
extern crate p2p;
extern crate params;
extern crate parking_lot;
extern crate pretty_env_logger;
extern crate primitives;
extern crate rpc;
extern crate script;
extern crate serialization as ser;
extern crate shrust;
extern crate sync;
extern crate verification;

#[macro_use]
extern crate log;

use clap::*;

use memory_pool::MemoryPool;
use params::NetworkParams;
use parking_lot::RwLock;
use std::process;
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;

mod config;
mod db_utils;
mod executor;
mod input_listener;
mod service;
mod wallet;
mod wallet_manager;

use executor::Executor;
use executor::Task as ExecutorTask;
use futures_cpupool::CpuPool;
use input_listener::InputListener;
use p2p::NetworkNode;
use service::Service;
use sync::{Acceptor, MessageHandler, MessageWrapper, Responder};
use wallet_manager::WalletManager;

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
                .help("Indicates this node will be bootstraped from")
        )
        .arg(
            Arg::with_name("number")
                .short("n")
                .long("number")
                .help("Number for node unique database for debug usage. Allows have databases inside different folders and unique telnet ports to communicate")
                .takes_value(true)
        )
        .arg(
            Arg::with_name("testnet")
                .short("t")
                .long("testnet")
                .help("Use testnet rules where tokens have no real world value")
        )
        .get_matches();

    let config = config::parse(&matches).expect("Could not parse command line arguments");

    //setup database
    let db_path_string = "./db".to_owned() + matches.value_of("number").unwrap_or("") + "/";
    let default_db_cache = 512;
    let storage = db_utils::open_db(db_path_string, default_db_cache);
    db_utils::init_db(storage.clone(), NetworkParams::Mainnet).unwrap(); //init db with genesis block

    //setup mempool
    let mempool_ref = Arc::new(RwLock::new(MemoryPool::new()));

    //setup cross thread communication channels
    let (to_network_sender, to_network_receiver) = mpsc::channel();
    let (from_network_sender, from_network_receiver) = mpsc::channel();
    let (responder_task_sender, responder_task_receiver) = mpsc::channel();
    let (terminate_sender, terminate_receiver) = mpsc::channel();
    let (executor_sender, executor_receiver) = mpsc::channel();
    let (wallet_manager_sender, wallet_manager_receiver) = mpsc::channel();

    let message_wrapper = MessageWrapper::new(config.network, to_network_sender.clone());

    let cpupool = CpuPool::new_num_cpus();

    //setup network requests responder
    let responder = Responder {
        storage: storage.clone(),
        task_receiver: responder_task_receiver,
        message_wrapper: message_wrapper.clone(),
    };

    let acceptor = Arc::new(Acceptor::new(
        mempool_ref.clone(),
        storage.clone(),
        config.network,
        cpupool
    ));

    //setup network messages handler
    let mut message_handler = MessageHandler::new(
        storage.clone(),
        from_network_receiver,
        responder_task_sender,
        acceptor,
        message_wrapper.clone(),
        config.network,
    );

    //setup p2p layer
    let mut network = NetworkNode::new(
        config.is_first,
        from_network_sender,
        to_network_receiver,
        terminate_receiver,
    );

    //setup wallet task and miscellaneous task executor
    let mut wallet_manager = WalletManager::new(
        mempool_ref.clone(),
        storage.clone(),
        wallet_manager_receiver,
        message_wrapper.clone(),
    );
    let mut executor = Executor::new(
        mempool_ref.clone(),
        storage.clone(),
        executor_receiver,
        message_wrapper.clone(),
    );

    //setup telnet listener
    let input_listener = InputListener::new(
        config.telnet_port,
        executor_sender.clone(),
        wallet_manager_sender,
        terminate_sender,
    );

    //launch services in different threads //TODO named threads
    let input_listener_thread = thread::spawn(move || input_listener.run());
    let responder_thread = thread::spawn(move || responder.run());
    let executor_thread = thread::spawn(move || executor.run());
    //let acceptor_thread = thread::spawn(move || acceptor.run());
    let wallet_manager_thread = thread::spawn(move || wallet_manager.run());
    let message_handler_thread = thread::spawn(move || message_handler.run());

    //prepare to handle Ctrl-C
    ctrlc::set_handler(move || {
        info!("Interrupted. Your blockchain latest state may not be saved. Please use `quit` command to exit properly");
        process::exit(0);
        //TODO send interrupt to input_listener and network_node, so we can exit properly even without `quit` command
        //interrupt_sender.send(true).expect("Could not exit properly. Blockchain latest state may be not saved");
    }).expect("Error setting Ctrl-C handler");

    network.set_on_connect_handler(move || {
        executor_sender
            .send(ExecutorTask::RequestLatestBlocks())
            .unwrap();
    });

    network.run(); //main thread loop
    drop(network); //remove everything after network loop has finished

    info!("Node is about to finish. If it doesn't it means one of the threads hangs and database won't save");

    //wait for other threads to finish
    input_listener_thread.join().unwrap();
    message_handler_thread.join().unwrap();
    wallet_manager_thread.join().unwrap();
    responder_thread.join().unwrap();
    executor_thread.join().unwrap();

    //TODO ending app properly is shallow. Every module and thread has to end for database to save properly
    //for this to happen every used Sender should be deleted so every thread may break its loop when no senders are available
    //maybe it's worth switching to some kind of per task futures and cpupool
    //Workaround TODO is to count every sender, so it's easier to determine which ones are hanging because they were cloned excessively
}
