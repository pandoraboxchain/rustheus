extern crate routing;
extern crate clap;
extern crate lru_time_cache;
extern crate maidsafe_utilities;
extern crate env_logger;
extern crate shrust;

#[macro_use] extern crate log;
#[macro_use] extern crate unwrap;
#[macro_use] extern crate serde_derive;

use lru_time_cache::LruCache;
use maidsafe_utilities::serialisation::{deserialise, serialise};
use routing::{Authority, ClientError, Event, EventStream, ImmutableData,
              MessageId, MutableData, Node, Prefix, Request, Response,
              Config, DevConfig, XorName};
use std::collections::HashMap;
use std::time::Duration;
use clap::*;
use std::thread;
use shrust::{Shell, ShellIO};
use std::net::TcpListener;
use std::io::Write;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, TryRecvError};

fn main() {
    env_logger::init().unwrap();
    let matches = App::new("simple_node")
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

    let first_node = matches.is_present("first");
    
    let mut node = ExampleNode::new(first_node);
    node.run();
}

fn handle_input_event(node: &mut Node, input: &Vec<u8>)
{
    send_a_message(node, &input);
}

fn send_a_message(node: &mut Node, message: &Vec<u8>)
{
    let node_name = *unwrap!(node.id()).name();
    let src = Authority::ManagedNode(node_name);
    let dst = Authority::NodeManager(node_name);

    unwrap!(node.send_put_idata_request(
        src,
        dst, 
        ImmutableData::new(message.clone()),
        MessageId::new()
    ));

    info!("Send a message from {:?} to its node manager saying {:?}", node_name, message);
}

fn create_shell(first_node: bool) -> Receiver<Vec<u8>>
{    
    let port = if first_node { "1234" } else { "1235" };
    info!("Node is about to start. You may now run $ telnet localhost {}", port);
    
    let (sender, receiver) = mpsc::channel();   

    let mut shell = Shell::new(sender);
    shell.new_command_noargs("hello", "Say 'hello' to the world", |io, _| {
        try!(writeln!(io, "Hello World !!!"));
        Ok(())
    });
    shell.new_command("message", "Send a message", 1, |io, sender, args| {
        let message = args[0].to_string();
        try!(writeln!(io, "message `{}` sent", message));
        let bytes = message.into_bytes();
        sender.send(bytes).unwrap();
        Ok(())
    });

    let serv = TcpListener::bind(String::from("0.0.0.0:") + port).expect("Cannot open socket");
    serv.set_nonblocking(true).expect("Cannot set non-blocking");

    thread::spawn(move || 
    {
        for stream in serv.incoming() {
        match stream {
                Ok(stream) => 
                {
                    let mut shell = shell.clone();
                    let mut io = ShellIO::new_io(stream);
                    shell.run_loop(&mut io);
                }
                Err(_) =>
                { 
                    //error!("{}", e);  
                }
            }
        }
    });

    return receiver;
}

/// A simple example node implementation for a network based on the Routing library.
pub struct ExampleNode {
    /// The node interface to the Routing library.
    node: Node,
    idata_store: HashMap<XorName, ImmutableData>,
    client_accounts: HashMap<XorName, u64>,
    request_cache: LruCache<MessageId, (Authority<XorName>, Authority<XorName>)>,
    first: bool,
    input_listener: Option<Receiver<Vec<u8>>>
}

impl ExampleNode {
    /// Creates a new node and attempts to establish a connection to the network.
    pub fn new(first: bool) -> ExampleNode {
        let dev_config = DevConfig { allow_multiple_lan_nodes: true, ..Default::default() };
        let config = Config { dev: Some(dev_config) };
        let node = unwrap!(Node::builder().first(first).config(config).create());

        ExampleNode {
            node: node,
            idata_store: HashMap::new(),
            client_accounts: HashMap::new(),
            request_cache: LruCache::with_expiry_duration(Duration::from_secs(60 * 10)),
            first: first,
            input_listener: None
        }
    }

    pub fn run_input_loop(&mut self)
    {
        let ref listener = self.input_listener.as_ref().unwrap();
        while let Ok(input) = listener.recv()
        {
            send_a_message(&mut self.node, &input);
        }
    }


    fn run(&mut self)
    {
        let mut disconnected = false;
        while !disconnected
        {
            if let Some(ref listener) = self.input_listener
            {
                if let Ok(ref input) = listener.recv()
                {
                    handle_input_event(&mut self.node, input);
                }
            }

            match self.node.try_next_ev() {
                Ok(event) => {
                    disconnected = !self.handle_node_event(event)
                },
                Err(error) => if error == TryRecvError::Disconnected { disconnected = true }
            }
            thread::sleep(Duration::from_millis(400));  //TODO make select! macro to wait for recv any of two threads
        }
    }
    /// Runs the event loop, handling events raised by the Routing library.
    fn handle_node_event(&mut self, event: Event) -> bool
    {  
        match event {
            Event::Request { request, src, dst } => self.handle_request(request, src, dst),
            Event::Response { response, src, dst } => self.handle_response(response, src, dst),
            Event::NodeAdded(name, _routing_table) => {
                info!(
                    "{} Received NodeAdded event {:?}",
                    self.get_debug_name(),
                    name
                );
                self.input_listener = Some(create_shell(self.first));
                self.handle_node_added(name);
            }
            Event::NodeLost(name, _routing_table) => {
                info!(
                    "{} Received NodeLost event {:?}",
                    self.get_debug_name(),
                    name
                );
            }
            Event::Connected => {
                info!("{} Received connected event", self.get_debug_name());
            }
            Event::Terminate => {
                info!("{} Received Terminate event", self.get_debug_name());
                return false;
            }
            Event::RestartRequired => {
                info!("{} Received RestartRequired event", self.get_debug_name());
                self.node = unwrap!(Node::builder().create());
            }
            Event::SectionSplit(prefix) => {
                info!(
                    "{} Received SectionSplit event {:?}",
                    self.get_debug_name(),
                    prefix
                );
                self.handle_split(prefix);
            }
            Event::SectionMerge(prefix) => {
                info!(
                    "{} Received SectionMerge event {:?}",
                    self.get_debug_name(),
                    prefix
                );
                let pfx = Prefix::new(prefix.bit_count() + 1, *unwrap!(self.node.id()).name());
                self.send_refresh(MessageId::from_lost_node(pfx.lower_bound()));
            }
            Event::Tick =>
            {
                info!("Tick");
            }
        }
        return true;
    }

    fn handle_request(
        &mut self,
        request: Request,
        src: Authority<XorName>,
        dst: Authority<XorName>,
    ) {
        match request {
            Request::Refresh(payload, msg_id) => self.handle_refresh(payload, msg_id),
            Request::GetIData { name, msg_id } => {
                self.handle_get_idata_request(src, dst, name, msg_id)
            }
            Request::PutIData { data, msg_id } => {
                self.handle_put_idata_request(src, dst, data, msg_id)
            }
            Request::GetMDataShell    { .. } |
            Request::ListMDataEntries { .. } |
            Request::GetMDataValue    { .. }
             => warn!("Received mutable request. No mutable database should be implemented for these nodes"),
            _ => {
                warn!(
                    "{:?} ExampleNode: handle for {:?} unimplemented.",
                    self.get_debug_name(),
                    request
                );
            }
        }
    }

    fn handle_response(
        &mut self,
        response: Response,
        _src: Authority<XorName>,
        dst: Authority<XorName>,
    ) {
        match (response, dst) {
            (Response::PutIData { res, msg_id }, Authority::NodeManager(_)) |
            (Response::PutIData { res, msg_id }, Authority::ManagedNode(_)) => {
                if let Some((src, dst)) = self.request_cache.remove(&msg_id) {
                    unwrap!(self.node.send_put_idata_response(src, dst, res, msg_id));
                }
            }
            (Response::PutMData { .. }, Authority::NodeManager(_)) |
            (Response::PutMData { .. }, Authority::ManagedNode(_)) => 
                warn!("Attempt to use response on mutable data request"),
            _ => unreachable!(),
        }
    }

    fn handle_get_idata_request(
        &mut self,
        src: Authority<XorName>,
        dst: Authority<XorName>,
        name: XorName,
        msg_id: MessageId,
    ) {
        match (src, dst) {
            (src @ Authority::Client { .. }, dst @ Authority::NaeManager(_)) => {
                let res = if let Some(data) = self.idata_store.get(&name) {
                    info!("data received is {:?}", data);
                    Ok(data.clone())
                } else {
                    info!(
                        "{:?} GetIData request failed for {:?}.",
                        self.get_debug_name(),
                        name
                    );
                    Err(ClientError::NoSuchData)
                };
                unwrap!(self.node.send_get_idata_response(dst, src, res, msg_id))
            }
            (src, dst) => unreachable!("Wrong Src and Dest Authority {:?} - {:?}", src, dst),
        }
    }

    fn handle_put_idata_request(
        &mut self,
        src: Authority<XorName>,
        dst: Authority<XorName>,
        data: ImmutableData,
        msg_id: MessageId,
    ) {
        match dst {
            Authority::NaeManager(_) => {
                info!(
                    "{:?} Storing : key {:?}, value {:?} sent from {:?}",
                    self.get_debug_name(),
                    data.name(),
                    data.value(),
                    src.name()
                );
                let _ = self.idata_store.insert(*data.name(), data);
                let _ = self.node.send_put_idata_response(dst, src, Ok(()), msg_id);
            }
            Authority::NodeManager(_) |
            Authority::ManagedNode(_) |
            Authority::ClientManager(_) => {
                info!(
                    "{:?} Put Request: Updating ClientManager: key {:?}, value {:?}",
                    self.get_debug_name(),
                    data.name(),
                    data.value()
                );
                if self.request_cache.insert(msg_id, (dst, src)).is_none() {
                    let src = dst;
                    let dst = Authority::NaeManager(*data.name());
                    unwrap!(self.node.send_put_idata_request(src, dst, data, msg_id));
                } else {
                    warn!("Attempt to reuse message ID {:?}.", msg_id);
                    unwrap!(self.node.send_put_idata_response(
                        dst,
                        src,
                        Err(ClientError::InvalidOperation),
                        msg_id,
                    ));
                }

            }
            _ => unreachable!("ExampleNode: Unexpected dst ({:?})", dst),
        }
    }

    fn handle_node_added(&mut self, name: XorName) {
        self.send_refresh(MessageId::from_added_node(name));
    }

    fn handle_split(&mut self, prefix: Prefix<XorName>) {
        let deleted_clients: Vec<_> = self.client_accounts
            .iter()
            .filter(|&(client_name, _)| !prefix.matches(client_name))
            .map(|(client_name, _)| *client_name)
            .collect();
        for client in &deleted_clients {
            let _ = self.client_accounts.remove(client);
        }

        let deleted_data: Vec<_> = self.idata_store
            .iter()
            .filter(|&(name, _)| !prefix.matches(name))
            .map(|(name, _)| *name)
            .collect();
        for id in &deleted_data {
            let _ = self.idata_store.remove(id);
        }
    }

    fn send_refresh(&mut self, msg_id: MessageId) {
        for (client_name, stored) in &self.client_accounts {
            let content = RefreshContent::Account {
                client_name: *client_name,
                data: *stored,
            };
            let content = unwrap!(serialise(&content));
            let auth = Authority::ClientManager(*client_name);
            unwrap!(self.node.send_refresh_request(auth, auth, content, msg_id));
        }

        for data in self.idata_store.values() {
            let refresh_content = RefreshContent::ImmutableData(data.clone());
            let content = unwrap!(serialise(&refresh_content));
            let auth = Authority::NaeManager(*data.name());
            unwrap!(self.node.send_refresh_request(auth, auth, content, msg_id));
        }
    }

    /// Receiving a refresh message means that a quorum has been reached: Enough other members in
    /// the section agree, so we need to update our data accordingly.
    fn handle_refresh(&mut self, content: Vec<u8>, _id: MessageId) {
        match unwrap!(deserialise(&content)) {
            RefreshContent::Account { client_name, data } => {
                info!(
                    "{:?} handle_refresh for account. client name: {:?}",
                    self.get_debug_name(),
                    client_name
                );
                let _ = self.client_accounts.insert(client_name, data);
            }
            RefreshContent::ImmutableData(data) => {
                info!(
                    "{:?} handle_refresh for immutable data. name: {:?}",
                    self.get_debug_name(),
                    data.name()
                );
                let _ = self.idata_store.insert(*data.name(), data);
            }
            RefreshContent::MutableData(_) => { }
        }
    }

    fn get_debug_name(&self) -> String {
        match self.node.id() {
            Ok(id) => format!("Node({:?})", id.name()),
            Err(err) => {
                error!("Could not get node name - {:?}", err);
                "Node(unknown)".to_owned()
            }
        }
    }
}

/// Refresh messages.
#[derive(Serialize, Deserialize)]
enum RefreshContent {
    Account { client_name: XorName, data: u64 },
    ImmutableData(ImmutableData),
    MutableData(MutableData),
}

