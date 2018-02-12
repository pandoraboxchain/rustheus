use keys::Address;

#[derive(Debug, PartialEq)]
pub enum Task
{
	CreateWallet(),
	SendCash(Address, u32)
}