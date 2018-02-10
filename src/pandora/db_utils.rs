use std::sync::Arc;
use std::path::PathBuf;
use std::fs::create_dir_all;
use db;
use chain::IndexedBlock;
use params::NetworkParams;

pub fn open_db(data_dir: String, db_cache: usize) -> db::SharedStore
{
	Arc::new(db::BlockChainDatabase::open_at_path(data_dir, db_cache).expect("Failed to open database"))
}

pub fn init_db(db: db::SharedStore, params: NetworkParams) -> Result<(), String>
{
	// insert genesis block if db is empty
	let genesis_block: IndexedBlock = params.genesis_block().into();
	match db.block_hash(0) {
		Some(ref db_genesis_block_hash) if db_genesis_block_hash != genesis_block.hash() => Err("Trying to open database with incompatible genesis block".into()),
		Some(_) => Ok(()),
		None => {
			let hash = genesis_block.hash().clone();
			db.insert(genesis_block).expect("Failed to insert genesis block to the database");
			db.canonize(&hash).expect("Failed to canonize genesis block");
			Ok(())
		}
	}
}

fn custom_path(data_dir: &str, sub_dir: &str) -> PathBuf {
	let mut path = PathBuf::from(data_dir);
	path.push(sub_dir);
	create_dir_all(&path).expect("Failed to get app dir");
	path
}
