use termchat::application::{Application, Config};

use clap::{App, Arg};

use std::net::{SocketAddrV4};

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
                .default_value("0")
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
                .default_value(&os_username)
                .help("Name used as user idenfication"),
        )
        .get_matches();

    // The next unwraps are safe because we specified a default value and a validator
    let config = Config {
        discovery_addr: matches.value_of("discovery").unwrap().parse().unwrap(),
        tcp_server_port: matches.value_of("tcp_server_port").unwrap().parse().unwrap(),
        user_name: matches.value_of("username").unwrap().into(),
    };

    let result = match Application::new(&config) {
        Ok(mut app) => app.run(),
        Err(e) => Err(e),
    };

    if let Err(e) = result {
        // app is now dropped we can print to stderr safely
        eprintln!("termchat exited with error: {}", e);
    }
}
