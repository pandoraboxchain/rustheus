use shrust::{Shell, ShellIO};
use std::net::TcpListener;
use std::thread;
use std::sync::mpsc::{self, Sender};
use std::io::Write;
use executor_tasks::Task;

type CommandAndArgs = (String, Vec<String>);

pub struct InputListener
{
    //shell: Shell<Sender<Task>>,
    //executor: Sender<Task>
}

impl InputListener
{
    pub fn new(is_first_node: bool, executor: Sender<Task>) -> Self
    {
        let shell = Self::create_shell(is_first_node);
        InputListener { }
    }

    fn create_shell(is_first_node: bool)
    {    
        let port = if is_first_node { "1234" } else { "1235" };
        info!("Node is about to start. You may now run $ telnet localhost {}", port);
        
        let (sender, receiver) = mpsc::channel();   

        let mut shell = Shell::new(sender);
        shell.new_command_noargs("hello", "Say 'hello' to the world", |io, _| {
            try!(writeln!(io, "Hello World !!!"));
            Ok(())
        });
        shell.new_command("transfer", "Transfer some money", 1, |_, executor, args|
        {
            let task = Task::CreateExampleTransaction(args[0].parse().unwrap());
            executor.send(task);
            Ok(())
        });
        shell.new_command("blocksign", "Sign block with all known transactions", 0, |_, executor, args|
        {
            let task = Task::SignBlock();
            executor.send(task);
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

    fn handle_cli_command(command: String, sender: &mut mpsc::Sender<CommandAndArgs>,  args: &[&str])
    {
        let mut arguments: Vec<String> = vec![];
        for argument in args
        {
            arguments.push(argument.to_string());
        }
        sender.send((command, arguments)).unwrap();
    }
}