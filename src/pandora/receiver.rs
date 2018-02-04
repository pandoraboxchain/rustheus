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
use std::time::{SystemTime, UNIX_EPOCH};
use chain::{BlockHeader, Block, Transaction, TransactionInput, TransactionOutput, OutPoint};
use crypto::DHash256;
use ser::{deserialize, serialize, serialize_with_flags, SERIALIZE_TRANSACTION_WITNESS};
use bytes::Bytes;

type CommandAndArgs = (String, Vec<String>);
type Mempool = Vec<Transaction>;

fn handle_transaction(&self, data: &Vec<u8>)
{
    self.received_bytes_tx.send(data);
}
