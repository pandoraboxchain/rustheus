#[derive(Debug, PartialEq)]
pub enum Task
{
	SignBlock(),
	CreateExampleTransaction(String)
}