use shrust::{ExecError, Shell, ShellIO};
use std::net::TcpListener;
use std::sync::mpsc::Sender;
use std::str::FromStr;
use executor::Task as ExecutorTask;
use keys::{Address, Private};
use wallet_manager::Task as WalletTask;
use primitives::hash::H256;

//TODO please find a way to do this better. This tuple is needed to access senders from command closures
type Senders = (Sender<ExecutorTask>, Sender<WalletTask>, Sender<bool>);

pub struct InputListener {
    node_number: u32,
    shell: Shell<Senders>,
}

impl InputListener {
    pub fn new(
        node_number: u32,
        executor: Sender<ExecutorTask>,
        wallet_manager: Sender<WalletTask>,
        terminator: Sender<bool>,
    ) -> Self {
        let shell = Self::create_shell(executor, wallet_manager, terminator);
        InputListener { node_number, shell }
    }

    fn create_shell(
        executor: Sender<ExecutorTask>,
        wallet_manager: Sender<WalletTask>,
        terminator: Sender<bool>,
    ) -> Shell<Senders> {
        let senders = (executor, wallet_manager, terminator);

        let mut shell = Shell::new(senders);
        shell.new_command(
            "blocksign",
            "Sign block with all known transactions",
            1,
            |_, senders, args| {
                let ref executor = senders.0;
                match Address::from_str(args[0]) {
                    Ok(coinbase_recipient) => {
                        executor.send(ExecutorTask::SignBlock(coinbase_recipient))?
                    }
                    Err(err) => error!("Can't parse address: {}", err),
                }
                Ok(())
            },
        );
        shell.new_command(
            "walletcreate",
            "Create address and show private and public keys",
            0,
            |_, senders, _| {
                let ref wallet_manager = senders.1;
                info!("Creating wallet...");
                wallet_manager.send(WalletTask::CreateWallet())?;
                Ok(())
            },
        );
        shell.new_command(
            "walletload",
            "Load wallet from provided private key",
            1,
            |_, senders, args| {
                let ref wallet_manager = senders.1;
                match Private::from_str(args[0]) {
                    Ok(private) => {
                        let task = WalletTask::LoadWallet(private);
                        info!("Loading wallet...");
                        wallet_manager.send(task)?;
                        Ok(())
                    }
                    //Err(err) => Err(ExecError::Other(Box::new(err)))
                    Err(err) => {
                        error!("Can't parse private key: {}", err);
                        Ok(()) //TODO find a way to return proper error
                    }
                }
            },
        );
        shell.new_command(
            "balance",
            "Show balance of currently loaded wallet",
            0,
            |_, senders, _| {
                let ref wallet_manager = senders.1;
                let task = WalletTask::CalculateBalance();
                wallet_manager.send(task)?;
                Ok(())
            },
        );
        shell.new_command(
            "transfer",
            "Transfer <address> <amount>",
            2,
            |_, senders, args| {
                let ref wallet_manager = senders.1;
                match Address::from_str(args[0]) {
                    Ok(address) => match args[1].parse::<u64>() {
                        Ok(amount) => {
                            let task = WalletTask::SendCash(address, amount);
                            wallet_manager.send(task)?;
                        }
                        Err(err) => error!("Can't parse amount: {}", err),
                    },
                    Err(err) => {
                        error!("Can't parse address: {}", err);
                    }
                }
                Ok(())
            },
        );
        shell.new_command(
            "shutdown",
            "Save database and shutdown properly",
            0,
            |_, senders, _| {
                let ref terminator = senders.2;
                terminator.send(true)?;
                Err(ExecError::Quit)
            },
        );
        shell.new_command(
            "txmeta",
            "Get transaction meta data for debug",
            1,
            |_, senders, args| {
                let ref executor = senders.0;
                match H256::from_str(args[0]) {
                    Ok(hash) => {
                        executor.send(ExecutorTask::GetTransactionMeta(hash))?;
                        Ok(())
                    }
                    Err(err) => {
                        error!("Can't parse hash: {}", err);
                        Ok(()) //TODO find a way to return proper error
                    }
                }
            },
        );
        shell.new_command(
            "blockhash",
            "Get block hash at height for debug",
            1,
            |_, senders, args| {
                let ref executor = senders.0;
                match args[0].parse::<u32>() {
                    Ok(block_height) => {
                        executor.send(ExecutorTask::GetBlockHash(block_height))?;
                        Ok(())
                    }
                    Err(err) => {
                        error!("Can't parse block height: {}", err);
                        Ok(())
                    }
                }
            },
        );

        shell
    }

    pub fn run(&self) {
        let port = "407".to_owned() + &self.node_number.to_string();
        info!(
            "Node is about to start. You may run $ telnet localhost {}",
            port
        );

        let serv = TcpListener::bind(String::from("0.0.0.0:") + &port).expect("Cannot open socket");
        serv.set_nonblocking(true).expect("Cannot set non-blocking");

        for stream in serv.incoming() {
            match stream {
                Ok(stream) => {
                    let mut shell = self.shell.clone();
                    let mut io = ShellIO::new_io(stream);
                    shell.run_loop(&mut io);
                    break; //TODO halt node as soon as we exit telnet for now
                }
                Err(_) => {}
            }
        }

        debug!("input listener thread ended");
    }
}
