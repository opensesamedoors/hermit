use clap::{Arg, Command, value_parser};

use crate::Client;
use crate::client::client::Mode;

pub fn create_cmd(client: &Client) -> Command {
    let cmd = Command::new("client")
        .about("Hermit C2 client")
        .allow_external_subcommands(true);

    match client.mode {
        Mode::Root => {
            cmd
                .subcommand(Command::new("exit")
                    .about("Close the connection and exit.")
                )
                // Listeners
                .subcommand(Command::new("listener")
                    .about("Manage listeners.")
                    .subcommand(Command::new("add")
                        .about("Add a new listener.")
                        .args([
                            Arg::new("protocol")
                                .help("Protocol")
                                .default_value("http")
                                .value_parser(value_parser!(String)),
                            Arg::new("host")
                                .short('H')
                                .long("host")
                                .help(format!("Host [default: {}]", client.server_host.to_string()))
                                .value_parser(value_parser!(String)),
                            Arg::new("port")
                                .short('P')
                                .long("port")
                                .help("Port")
                                .required(true)
                                .value_parser(value_parser!(u16)),
                            Arg::new("name")
                                .short('n')
                                .long("name")
                                .help("Specify the name of a listener")
                                .value_parser(value_parser!(String)),
                        ])
                    )
                    .subcommand(Command::new("delete")
                        .about("Delete a listener.")
                        .arg(Arg::new("listener")
                            .help("Listener ID or name to delete")
                            .required(true)
                            .value_parser(value_parser!(String))
                        )                    
                    )
                    .subcommand(Command::new("start")
                        .about("Start a listener.")
                        .arg(Arg::new("listener")
                            .help("Listener ID or name to start")
                            .required(true)
                            .value_parser(value_parser!(String))
                        )
                    )
                    .subcommand(Command::new("stop")
                        .about("Stop a listener.")
                        .arg(Arg::new("listener")
                            .help("Listener ID or name to stop")
                            .required(true)
                            .value_parser(value_parser!(String))
                        )
                    )
                    .subcommand(Command::new("list")
                        .about("List listeners.")
                    )
                )
                .subcommand(Command::new("listeners")
                    .about("List listeners.")
                )
                // Agents
                .subcommand(Command::new("agent")
                    .about("Manage agents.")
                    .subcommand(Command::new("interact")
                        .about("Interact with the specified agent.")
                        .arg(Arg::new("name")
                            .help("Agent ID or name")
                            .required(true)
                            .value_parser(value_parser!(String)))
                    )
                    .subcommand(Command::new("list")
                        .about("List agents.")
                    )
                )
                .subcommand(Command::new("agents")
                    .about("List agents.")
                )
                // Implants
                .subcommand(Command::new("implant")
                    .about("Manage implants.")
                    .subcommand(Command::new("gen")
                        .about("Generate a new implant.")
                        .args([
                            Arg::new("name")
                                .short('n')
                                .long("name")
                                .help("Set an implant name")
                                .value_parser(value_parser!(String)),
                            Arg::new("listener")
                                .short('l')
                                .long("listener")
                                .help("Listener URL that an agent connect to")
                                .default_value("http://127.0.0.1:8000/")
                                .value_parser(value_parser!(String)),
                            Arg::new("os")
                                .short('o')
                                .long("os")
                                .help("Target OS")
                                .default_value("windows")
                                .value_parser(value_parser!(String)),
                            Arg::new("arch")
                                .short('a')
                                .long("arch")
                                .help("Target architecture")
                                .default_value("amd64")
                                .value_parser(value_parser!(String)),
                            Arg::new("format")
                                .short('f')
                                .long("format")
                                .help("File format to be generated")
                                .default_value("exe")
                                .value_parser(value_parser!(String)),
                            Arg::new("sleep")
                                .short('s')
                                .long("sleep")
                                .help("Sleep time for each request to listener")
                                .default_value("3")
                                .value_parser(value_parser!(u64)),
                        ])
                    )
                    .subcommand(Command::new("download")
                        .about("Download specified implant.")
                        .arg(Arg::new("name")
                            .help("Implant ID or name.")
                            .required(true))
                    )
                    .subcommand(Command::new("list")
                        .about("List implants generated.")
                    )
                )
                .subcommand(Command::new("implants")
                    .about("List implants generated.")
                )
        }
        Mode::Agent(_) => {
            cmd
                .subcommand(Command::new("exit")
                    .about("Exit the agent mode.")
                )
                // Tasks
                .subcommand(Command::new("screenshot")
                    .about("Take a screenshot for target machine.")
                )
                .subcommand(Command::new("shell")
                    .about("Execute shell command for target machine.")
                    .arg(Arg::new("command")
                        .help("Specified command.")
                        .required(true)
                        .value_parser(value_parser!(String))
                    )
                )
        }
    }
}