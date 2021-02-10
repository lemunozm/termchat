use std::net::{SocketAddrV4};
use clap::ArgMatches;
use serde::{Serialize, Deserialize};
use crate::util::Result;

#[derive(Serialize, Deserialize)]
pub struct Config {
    pub discovery_addr: SocketAddrV4,
    pub tcp_server_port: u16,
    pub user_name: String,
    pub terminal_bell: bool,
}

impl Default for Config {
    fn default() -> Self {
        Config {
            discovery_addr: "238.255.0.1:5877".parse().unwrap(),
            tcp_server_port: "0".parse().unwrap(),
            user_name: whoami::username(),
            terminal_bell: true,
        }
    }
}

impl Config {
    /// Try to read config file from disk
    /// If it does not exist, create it with default config values, and return that
    /// If it fails for any other reason return None
    fn from_config_file() -> Option<Self> {
        let config_dir_path = dirs_next::config_dir()?.join("termchat");
        if let Err(e) = std::fs::create_dir_all(&config_dir_path) {
            if e.kind() != std::io::ErrorKind::AlreadyExists {
                return None
            }
        }
        let config_file_path = config_dir_path.join("config");

        let create_config = |config_file_path| -> Result<Config> {
            let config = Config::default();
            std::fs::write(config_file_path, toml::to_string(&config)?)?;
            Ok(config)
        };

        match std::fs::read_to_string(&config_file_path) {
            Ok(config) => toml::from_str(&config).ok(),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                // Config file was not found -> create it with default_values
                match create_config(&config_file_path) {
                    Ok(config) => Some(config),
                    Err(_) => None,
                }
            }
            Err(_) => None,
        }
    }

    /// Read configuration file from disk
    /// If it fails for any reason use default Config value
    /// If the user uses the cli arguments they will override the default values
    pub fn from_matches(matches: ArgMatches) -> Self {
        let mut config = Config::from_config_file().unwrap_or_default();

        // the next unwrap are safe because we use clap validator
        if let Some(discovery_addr) = matches.value_of("discovery") {
            config.discovery_addr = discovery_addr.parse().unwrap();
        }
        if let Some(tcp_server_port) = matches.value_of("tcp_server_port") {
            config.tcp_server_port = tcp_server_port.parse().unwrap();
        }
        if let Some(user_name) = matches.value_of("username") {
            config.user_name = user_name.parse().unwrap();
        }
        if matches.value_of("quiet-mode").is_some() {
            config.terminal_bell = false;
        }

        config
    }
}
