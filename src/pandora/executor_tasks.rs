use keys::Address;

#[derive(Debug, PartialEq)]
pub enum Task
{
	SignBlock(Address)
}