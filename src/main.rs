mod application;
mod state;
mod ui;
mod terminal_events;
mod util;

use application::{Application};

use clap::{App, Arg};

fn main() {
    let os_username = whoami::username();

    let matches = App::new(clap::crate_name!())
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about(clap::crate_description!())
        .arg(Arg::with_name("discovery")
            .long("discovery")
            .short("d")
            .default_value("238.255.0.1:5877")
            .help("Multicast address to found others 'termchat' applications"))
        .arg(Arg::with_name("username")
            .long("username")
            .short("u")
            .default_value(&os_username)
            .help("Name used as user idenfication"))
        .get_matches();

    let addr = match matches.value_of("discovery").unwrap().parse() {
        Ok(addr) => addr,
        Err(_) => return eprintln!("'discovery' must be a valid multicast address"),
    };

    let name = matches.value_of("username").unwrap();

    if let Ok(mut app) = Application::new(addr, &name) {
        app.run()
    }
}
