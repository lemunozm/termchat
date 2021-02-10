use termchat::application::{Application};
use termchat::config::Config;

use clap::{App, Arg};

use std::net::{SocketAddrV4};

fn main() {
    let matches = App::new(clap::crate_name!())
        .version(clap::crate_version!())
        .author(clap::crate_authors!())
        .about(clap::crate_description!())
        .arg(
            Arg::with_name("discovery")
                .long("discovery")
                .short("d")
                .validator(|addr| match addr.parse::<SocketAddrV4>() {
                    Ok(_) => Ok(()),
                    Err(_) => Err("The value must have syntax ipv4:port".into()),
                })
                .help("Multicast address to found others 'termchat' applications"),
        )
        .arg(
            Arg::with_name("tcp_server_port")
                .long("tcp-server-port")
                .short("t")
                .validator(|port| match port.parse::<u16>() {
                    Ok(_) => Ok(()),
                    Err(_) => Err("The value must be in range 0..65535".into()),
                })
                .help("Tcp server port used when communicating with other termchat instances"),
        )
        .arg(
            Arg::with_name("username")
                .long("username")
                .short("u")
                .help("Name used as user idenfication"),
        )
        .arg(
            Arg::with_name("quiet-mode")
                .long("quiet-mode")
                .short("q")
                .help("Disable the terminal bell sound"),
        )
        .get_matches();

    // The next unwraps are safe because we specified a default value and a validator
    let config = Config::from_matches(matches);

    let result = match Application::new(&config) {
        Ok(mut app) => app.run(std::io::stdout()),
        Err(e) => Err(e),
    };

    if let Err(e) = result {
        // app is now dropped we can print to stderr safely
        eprintln!("termchat exited with error: {}", e);
    }
}
