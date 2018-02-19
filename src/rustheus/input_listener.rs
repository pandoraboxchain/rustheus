use shrust::{Shell, ShellIO};
use std::net::TcpListener;
use std::thread;
use std::sync::mpsc::Sender;
use std::io::Write;
use std::str::FromStr;
use executor_tasks::Task;
use keys::{Private, Address};
use wallet_manager_tasks::Task as WalletTask;

pub struct InputListener
{
    //shell: Shell<Sender<Task>>,
    //executor: Sender<Task>
}

impl InputListener
{
    pub fn new(is_first_node: bool, executor: Sender<Task>, wallet_manager: Sender<WalletTask>) -> Self
    {
        let _shell = Self::create_shell(is_first_node, executor, wallet_manager);
        InputListener { }
    }

    fn create_shell(is_first_node: bool, executor: Sender<Task>, wallet_manager: Sender<WalletTask>)
    {    
        let port = if is_first_node { "1234" } else { "1235" };
        info!("Node is about to start. You may now run $ telnet localhost {}", port);

        let senders = (executor, wallet_manager);

        let mut shell = Shell::new(senders);
        shell.new_command_noargs("hello", "Say 'hello' to the world", |io, _| {
            try!(writeln!(io, "Hello World !!!"));
            Ok(())
        });
        shell.new_command("blocksign", "Sign block with all known transactions", 1, |_, senders, args|
        {
            let ref executor = senders.0;
            match Address::from_str(args[0])
            {
                Ok(coinbase_recipient) => executor.send(Task::SignBlock(coinbase_recipient))?,
                Err(err) => error!("Can't parse address: {}", err),
            }            
            Ok(())
        });
        shell.new_command("walletcreate", "Create address and show private and public keys", 0, |_, senders, _|
        {
            let ref wallet_manager = senders.1;
            info!("Creating wallet...");
            wallet_manager.send(WalletTask::CreateWallet())?;
            Ok(())
        });
        shell.new_command("walletload", "Load wallet from provided private key", 1, |_, senders, args|
        {
            let ref wallet_manager = senders.1;
            match Private::from_str(args[0])
            {
                Ok(private) => {
                    let task = WalletTask::LoadWallet(private);
                    info!("Loading wallet...");            
                    wallet_manager.send(task)?;
                    Ok(())
                },
                //Err(err) => Err(ExecError::Other(Box::new(err)))
                Err(err) => {
                    error!("Can't parse private key: {}", err);
                    Ok(())  //TODO find a way to return proper error
                }
            }
        });
        shell.new_command("balance", "Show balance of currently loaded wallet", 0, |_, senders, _|
        {
            let ref wallet_manager = senders.1;
            let task = WalletTask::CalculateBalance();           
            wallet_manager.send(task)?;
            Ok(())
        });
        shell.new_command("transfer", "Transfer <address> <amount>", 2, |_, senders, args|
        {
            let ref wallet_manager = senders.1;
            match Address::from_str(args[0])
            {
                Ok(address) => {
                    match args[1].parse::<u64>() {
                        Ok(amount) => {
                            let task = WalletTask::SendCash(address, amount);          
                            wallet_manager.send(task)?;
                        },
                        Err(err) => error!("Can't parse amount: {}", err)
                    }
                },
                Err(err) => {
                    error!("Can't parse address: {}", err);
                }
            }
            Ok(())
        });

        let serv = TcpListener::bind(String::from("0.0.0.0:") + port).expect("Cannot open socket");
        serv.set_nonblocking(true).expect("Cannot set non-blocking");

        thread::spawn(move || 
        {
            for stream in serv.incoming() {
            match stream {
                    Ok(stream) => 
                    {
                        let mut shell = shell.clone();
                        let mut io = ShellIO::new_io(stream);
                        shell.run_loop(&mut io);
                    }
                    Err(_) =>
                    { 
                        //error!("{}", e);  
                    }
                }
            }
        });

        //return shell;
    }
}