use shrust::{Shell, ShellIO};
use std::net::TcpListener;
use std::thread;
use std::sync::mpsc::Sender;
use std::io::Write;
use std::str::FromStr;
use executor_tasks::Task;
use keys::{Private};
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
        shell.new_command("transfer", "Transfer some money", 1, |_, senders, args|
        {
            let ref executor = senders.0;
            let task = Task::CreateExampleTransaction(args[0].parse().unwrap());
            info!("transfer command was send to executor");
            try!(executor.send(task));
            Ok(())
        });
        shell.new_command("blocksign", "Sign block with all known transactions", 0, |_, senders, _|
        {
            let ref executor = senders.0;
            let task = Task::SignBlock();
            info!("sign block command was send to executor");            
            executor.send(task)?;
            Ok(())
        });
        shell.new_command("walletcreate", "Create address and show private and public keys", 0, |_, senders, _|
        {
            let ref wallet_manager = senders.1;
            let task = WalletTask::CreateWallet();
            info!("Creating wallet...");            
            wallet_manager.send(task)?;
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
        shell.new_command("walletbalance", "Create address and show private and public keys", 0, |_, senders, _|
        {
            let ref wallet_manager = senders.1;
            let task = WalletTask::CalculateBalance();           
            wallet_manager.send(task)?;
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