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
use std::io::Write;

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

    //info!("shell is about to start");
    
    // let mut shell = Shell::new(());
    // shell.new_command_noargs("hello", "Say 'hello' to the world", |io, _| {
    //     try!(writeln!(io, "Hello World !!!"));
    //     Ok(())
    // });

    // shell.run_loop(&mut ShellIO::default());
}

/// A simple example node implementation for a network based on the Routing library.
pub struct ExampleNode {
    /// The node interface to the Routing library.
    node: Node,
    idata_store: HashMap<XorName, ImmutableData>,
    mdata_store: HashMap<(XorName, u64), MutableData>,
    client_accounts: HashMap<XorName, u64>,
    request_cache: LruCache<MessageId, (Authority<XorName>, Authority<XorName>)>,
    first: bool
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
            mdata_store: HashMap::new(),
            client_accounts: HashMap::new(),
            request_cache: LruCache::with_expiry_duration(Duration::from_secs(60 * 10)),
            first: first
        }
    }

    /// Runs the event loop, handling events raised by the Routing library.
    pub fn run(&mut self) {
        while let Ok(event) = self.node.next_ev() {
            match event {
                Event::Request { request, src, dst } => self.handle_request(request, src, dst),
                Event::Response { response, src, dst } => self.handle_response(response, src, dst),
                Event::NodeAdded(name, _routing_table) => {
                    info!(
                        "{} Received NodeAdded event {:?}",
                        self.get_debug_name(),
                        name
                    );
                    self.handle_node_added(name);
                    // if !self.first
                    // {
                    //     thread::sleep(Duration::from_secs(2));
                    //     self.send_a_message(name);
                    // }
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
                    break;
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
                    if !self.first
                    {
                        self.send_a_message_no_destination();
                    }
                    info!("Tick");
                    
                }
                event => {
                    info!("{} Received {:?} event", self.get_debug_name(), event);
                }
            }
        }
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
            Request::GetMDataShell { name, tag, msg_id } => {
                self.handle_get_mdata_shell_request(src, dst, name, tag, msg_id)
            }
            Request::ListMDataEntries { name, tag, msg_id } => {
                self.handle_list_mdata_entries_request(src, dst, name, tag, msg_id)
            }
            Request::GetMDataValue {
                name,
                tag,
                key,
                msg_id,
            } => self.handle_get_mdata_value_request(src, dst, name, tag, key, msg_id),
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
            (Response::PutMData { res, msg_id }, Authority::NodeManager(_)) |
            (Response::PutMData { res, msg_id }, Authority::ManagedNode(_)) => {
                if let Some((src, dst)) = self.request_cache.remove(&msg_id) {
                    unwrap!(self.node.send_put_mdata_response(src, dst, res, msg_id));
                }
            }
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
                    data
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

    fn handle_get_mdata_shell_request(
        &mut self,
        src: Authority<XorName>,
        dst: Authority<XorName>,
        name: XorName,
        tag: u64,
        msg_id: MessageId,
    ) {
        match (src, dst) {
            (src @ Authority::Client { .. }, dst @ Authority::NaeManager(_)) => {
                let res = if let Some(data) = self.mdata_store.get(&(name, tag)) {
                    Ok(data.shell())
                } else {
                    info!("{:?} GetMDataShell request failed for {:?}.",
                           self.get_debug_name(),
                           (name, tag));
                    Err(ClientError::NoSuchData)
                };

                unwrap!(self.node.send_get_mdata_shell_response(
                    dst,
                    src,
                    res,
                    msg_id,
                ))
            }
            (src, dst) => unreachable!("Wrong Src and Dest Authority {:?} - {:?}", src, dst),
        }
    }

    fn handle_list_mdata_entries_request(
        &mut self,
        src: Authority<XorName>,
        dst: Authority<XorName>,
        name: XorName,
        tag: u64,
        msg_id: MessageId,
    ) {
        match (src, dst) {
            (src @ Authority::Client { .. }, dst @ Authority::NaeManager(_)) => {
                let res = if let Some(data) = self.mdata_store.get(&(name, tag)) {
                    Ok(data.entries().clone())
                } else {
                    info!("{:?} ListMDataEntries request failed for {:?}.",
                           self.get_debug_name(),
                           (name, tag));
                    Err(ClientError::NoSuchData)
                };

                unwrap!(self.node.send_list_mdata_entries_response(
                    dst,
                    src,
                    res,
                    msg_id,
                ))
            }
            (src, dst) => unreachable!("Wrong Src and Dest Authority {:?} - {:?}", src, dst),
        }
    }

    fn handle_get_mdata_value_request(
        &mut self,
        src: Authority<XorName>,
        dst: Authority<XorName>,
        name: XorName,
        tag: u64,
        key: Vec<u8>,
        msg_id: MessageId,
    ) {
        match (src, dst) {
            (src @ Authority::Client { .. }, dst @ Authority::NaeManager(_)) => {
                let res = self.mdata_store
                    .get(&(name, tag))
                    .ok_or(ClientError::NoSuchData)
                    .and_then(|data| {
                        data.get(&key).cloned().ok_or(ClientError::NoSuchEntry)
                    })
                    .map_err(|error| {
                        info!("{:?} GetMDataValue request failed for {:?}.",
                                        self.get_debug_name(),
                                        (name, tag));
                        error
                    });

                unwrap!(self.node.send_get_mdata_value_response(
                    dst,
                    src,
                    res,
                    msg_id,
                ))
            }
            (src, dst) => unreachable!("Wrong Src and Dest Authority {:?} - {:?}", src, dst),
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

        let deleted_data: Vec<_> = self.mdata_store
            .iter()
            .filter(|&(&(ref name, _), _)| !prefix.matches(name))
            .map(|(id, _)| *id)
            .collect();
        for id in &deleted_data {
            let _ = self.mdata_store.remove(id);
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

        for data in self.mdata_store.values() {
            let content = RefreshContent::MutableData(data.clone());
            let content = unwrap!(serialise(&content));
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
            RefreshContent::MutableData(data) => {
                info!(
                    "{:?} handle_refresh for mutable data. name: {:?}, tag: {}",
                    self.get_debug_name(),
                    data.name(),
                    data.tag()
                );
                let _ = self.mdata_store.insert((*data.name(), data.tag()), data);
            }
        }
    }

    fn send_a_message(&mut self, destination_name: XorName )
    {
        let node_name = *unwrap!(self.node.id()).name();

        let src = Authority::ManagedNode(node_name);
        let dst = Authority::ManagedNode(destination_name);

        unwrap!(self.node.send_put_idata_request(
            src,
            dst, 
            ImmutableData::new(vec![1,2,3,4]),
            MessageId::new()
        ));

        info!("Send a message! from {:?} to {:?}", node_name, destination_name);
    }

    fn send_a_message_no_destination(&mut self)
    {
        let node_name = *unwrap!(self.node.id()).name();

        let src = Authority::ManagedNode(node_name);
        let dst = Authority::NodeManager(node_name);

        unwrap!(self.node.send_put_idata_request(
            src,
            dst, 
            ImmutableData::new(vec![1,2,3,4]),
            MessageId::new()
        ));

        info!("Send a message no dst! from {:?}", node_name);
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

