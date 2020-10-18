mod application;
mod state;
mod terminal_events;
mod ui;
mod util;

use application::Application;

use clap::{App, Arg};

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

fn main() {
    let os_username = whoami::username();

    let matches = App::new(clap::crate_name!())
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about(clap::crate_description!())
        .arg(
            Arg::with_name("discovery")
                .long("discovery")
                .short("d")
                .default_value("238.255.0.1:5877")
                .help("Multicast address to found others 'termchat' applications"),
        )
        .arg(
            Arg::with_name("tcp_server_port")
                .long("tcp-server-port")
                .short("t")
                .default_value("0")
                .help("Tcp server port used when communicating with other termchat instances"),
        )
        .arg(
            Arg::with_name("username")
                .long("username")
                .short("u")
                .default_value(&os_username)
                .help("Name used as user idenfication"),
        )
        .get_matches();

    // The next unwraps are safe because we specified a default value

    let discovery_addr = match matches.value_of("discovery").unwrap().parse() {
        Ok(discovery_addr) => discovery_addr,
        Err(_) => return eprintln!("'discovery' must be a valid multicast address"),
    };

    let tcp_server_port = match matches.value_of("tcp_server_port").unwrap().parse() {
        Ok(port) => port,
        Err(_) => return eprintln!("Unable to parse tcp server port"),
    };

    let name = matches.value_of("username").unwrap();

    match Application::new(discovery_addr, tcp_server_port, &name) {
        Ok(mut app) => {
            if let Err(e) = app.run() {
                application::clean_terminal();
                eprintln!("Termchat crashed, err: {}", e);
            }
        }
        Err(e) => {
            application::clean_terminal();
            eprintln!("Termchat crashed, err: {}", e);
        }
    }
}
