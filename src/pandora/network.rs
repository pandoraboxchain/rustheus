use lru_time_cache::LruCache;
use maidsafe_utilities::serialisation::{deserialise, serialise};
use routing::{Authority, ClientError, Event, EventStream, ImmutableData,
              MessageId, MutableData, Node, Prefix, Request, Response,
              Config, DevConfig, XorName};
use std::collections::HashMap;
use std::time::Duration;
use std::sync::mpsc::{self, Sender, Receiver, TryRecvError};
use chain::bytes::Bytes;
use std::thread;

/// A simple example node implementation for a network based on the Routing library.
pub struct NetworkNode {
    /// The node interface to the Routing library.
    node: Node,
    idata_store: HashMap<XorName, ImmutableData>,
    client_accounts: HashMap<XorName, u64>,
    request_cache: LruCache<MessageId, (Authority<XorName>, Authority<XorName>)>,

    received_bytes_listener: Sender<Bytes>,
    
    bytes_to_send_tx: Sender<Bytes>,  //public
    bytes_to_send_rx: Receiver<Bytes>  
}

impl NetworkNode {
    /// Creates a new node and attempts to establish a connection to the network.
    pub fn new(first: bool, received_bytes_listener: Sender<Bytes>) -> NetworkNode {
        let dev_config = DevConfig { allow_multiple_lan_nodes: true, ..Default::default() };
        let config = Config { dev: Some(dev_config) };
        let node = unwrap!(Node::builder().first(first).config(config).create());

        let (bytes_to_send_tx, bytes_to_send_rx) = mpsc::channel();

        NetworkNode {
            node: node,
            idata_store: HashMap::new(),
            client_accounts: HashMap::new(),
            request_cache: LruCache::with_expiry_duration(Duration::from_secs(60 * 10)),

            received_bytes_listener,   
            
            bytes_to_send_tx,  //public
            bytes_to_send_rx 
        }
    }

    pub fn run(&mut self)
    {
        let mut disconnected = false;
        while !disconnected
        {
            if let Ok(bytes_to_send) = self.bytes_to_send_rx.try_recv()
            {
                self.send_a_message(&bytes_to_send.take());
            }

            match self.node.try_next_ev() {
                Ok(event) => disconnected = !self.handle_node_event(event),
                Err(error) => if error == TryRecvError::Disconnected { disconnected = true }
            }
            thread::sleep(Duration::from_millis(400));  //TODO make select! macro to wait for recv any of two threads
        }
    }

    pub fn get_bytes_to_send_sender(&self) -> Sender<Bytes>
    {
        self.bytes_to_send_tx.clone()
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
                    "{:?} NetworkNode: handle for {:?} unimplemented.",
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
                    self.handle_message(data.value());
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
            _ => unreachable!("NetworkNode: Unexpected dst ({:?})", dst),
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

    fn handle_message(&mut self, data: &Vec<u8>)
    {
        self.received_bytes_listener.send(data.clone().into()).unwrap();
    }

    fn send_a_message(&mut self, message: &Vec<u8>)
    {
        let node_name = *unwrap!(self.node.id()).name();
        let src = Authority::ManagedNode(node_name);
        let dst = Authority::NodeManager(node_name);

        unwrap!(self.node.send_put_idata_request(
            src,
            dst, 
            ImmutableData::new(message.clone()),
            MessageId::new()
        ));

        info!("Send a message from {:?} to its node manager saying {:?}", node_name, message);
    }
}

/// Refresh messages.
#[derive(Serialize, Deserialize)]
enum RefreshContent {
    Account { client_name: XorName, data: u64 },
    ImmutableData(ImmutableData),
    MutableData(MutableData),
}


