use keys::Address;
use keys::Private;

#[derive(Debug, PartialEq)]
pub enum Task
{
	CreateWallet(),
	SendCash(Address, u64),
	LoadWallet(Private),
	CalculateBalance()
}