#[derive(Debug, PartialEq)]
pub enum Task
{
	CreateWallet(),
	SendCash(u32)
}

unsafe impl Send for Task {}
unsafe impl Sync for Task {}