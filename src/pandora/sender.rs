pub struct Sender
{
    node: &mut Node
}

impl Sender
{
    fn send(&self, message: &Vec<u8>)
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