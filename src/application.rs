use super::state::{ApplicationState, CursorMovement, LogMessage, MessageType, ScrollMovement};
use super::terminal_events::TerminalEventCollector;
use super::ui::{self};
use crate::util::{Error, Result};

use crossterm::event::{Event as TermEvent, KeyCode, KeyEvent, KeyModifiers};
use crossterm::{
    terminal::{self},
    ExecutableCommand,
};

use tui::backend::CrosstermBackend;
use tui::Terminal;

use message_io::events::EventQueue;
use message_io::network::{NetEvent, NetworkManager};

use serde::{Deserialize, Serialize};

use std::io::{self, Stdout};
use std::net::SocketAddr;

#[derive(Serialize, Deserialize)]
enum NetMessage {
    HelloLan(String, u16), // user_name, server_port
    HelloUser(String),     // user_name
    UserMessage(String),   // content
    UserData(String, Option<(Vec<u8>, usize)>, Option<String>), // file_name, data
}

enum Event {
    Network(NetEvent<NetMessage>),
    Terminal(TermEvent),
    // Close event with an optional error in case of failure
    // Close(None) means no error happened
    Close(Option<Error>),
}

pub struct Application {
    event_queue: EventQueue<Event>,
    network: NetworkManager,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    _terminal_events: TerminalEventCollector,
    discovery_addr: SocketAddr,
    tcp_server_addr: SocketAddr,
    user_name: String,
}

impl Application {
    pub fn new(
        discovery_addr: SocketAddr,
        tcp_server_port: u16,
        user_name: &str,
    ) -> Result<Application> {
        // Guard to make sure to cleanup if a failure happens in the next lines
        let _g = Guard;

        let mut event_queue = EventQueue::new();

        let sender = event_queue.sender().clone(); // Collect network events
        let network = NetworkManager::new(move |net_event| sender.send(Event::Network(net_event)));

        let sender = event_queue.sender().clone(); // Collect terminal events
        let _terminal_events = TerminalEventCollector::new(move |term_event| match term_event {
            Ok(event) => sender.send(Event::Terminal(event)),
            Err(e) => sender.send(Event::Close(Some(e))),
        })?;

        terminal::enable_raw_mode()?;
        io::stdout().execute(terminal::EnterAlternateScreen)?;
        let terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

        let tcp_server_addr = ([0, 0, 0, 0], tcp_server_port).into();

        std::mem::forget(_g);

        Ok(Application {
            event_queue,
            network,
            terminal,
            // Stored because we want its internal thread functionality until the Application was dropped
            _terminal_events,
            discovery_addr,
            tcp_server_addr,
            user_name: user_name.into(),
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let mut state = ApplicationState::new();
        ui::draw(&mut self.terminal, &state)?;

        let (_, server_addr) = self.network.listen_tcp(self.tcp_server_addr)?;
        let server_port = server_addr.port();

        self.network.listen_udp_multicast(self.discovery_addr)?;

        let discovery_endpoint = self.network.connect_udp(self.discovery_addr)?;

        let message = NetMessage::HelloLan(self.user_name.clone(), server_port);
        self.network.send(discovery_endpoint, message)?;

        loop {
            match self.event_queue.receive() {
                Event::Network(net_event) => match net_event {
                    NetEvent::Message(endpoint, message) => match message {
                        // by udp (multicast):
                        NetMessage::HelloLan(user, server_port) => {
                            let server_addr = (endpoint.addr().ip(), server_port);
                            if user != self.user_name {
                                let mut try_connect = || -> Result<()> {
                                    let user_endpoint = self.network.connect_tcp(server_addr)?;
                                    let message = NetMessage::HelloUser(self.user_name.clone());
                                    self.network.send(user_endpoint, message)?;
                                    state.connected_user(user_endpoint, &user);
                                    Ok(())
                                };
                                if let Err(e) = try_connect() {
                                    let message = LogMessage::new(
                                        String::from("termchat :"),
                                        MessageType::Error(e.to_string()),
                                    );
                                    state.add_message(message);
                                }
                            }
                        }
                        // by tcp:
                        NetMessage::HelloUser(user) => {
                            state.connected_user(endpoint, &user);
                        }
                        NetMessage::UserMessage(content) => {
                            if let Some(user) = state.user_name(endpoint) {
                                let message =
                                    LogMessage::new(user.into(), MessageType::Content(content));
                                state.add_message(message);
                            }
                        }
                        NetMessage::UserData(file_name, maybe_data, maybe_error) => {
                            use std::io::Write;
                            if state.user_name(endpoint).is_some() {
                                // safe unwrap due to check
                                let user = state.user_name(endpoint).unwrap().to_owned();

                                let try_write = || -> Result<()> {
                                    if let Some(error) = maybe_error {
                                        return Err(format!(
                                            "{} encountred an error while sending {}, error: {}",
                                            user, file_name, error
                                        )
                                        .into());
                                    }
                                    // if the error is none we know that maybe_data is some
                                    let (data, bytes_read) = maybe_data.unwrap();

                                    //done
                                    if bytes_read == 0 {
                                        return Ok(());
                                    }

                                    let path = std::env::temp_dir().join("termchat");
                                    let user_path = path.join(&user);
                                    // Ignore already exists error
                                    let _ = std::fs::create_dir_all(&user_path);
                                    let file_path = user_path.join(file_name);

                                    let mut file = std::fs::OpenOptions::new()
                                        .create(true)
                                        .append(true)
                                        .open(file_path)?;
                                    file.write_all(&data)?;
                                    Ok(())
                                };

                                if let Err(e) = try_write() {
                                    state.add_message(termchat_error_message(format!(
                                        "termchat: Failed to write data sent from user: {}",
                                        user
                                    )));
                                    state.add_message(termchat_error_message(e.to_string()));
                                }
                            }
                        }
                    },
                    NetEvent::AddedEndpoint(_) => (),
                    NetEvent::RemovedEndpoint(endpoint) => {
                        state.disconnected_user(endpoint);
                    }
                },
                Event::Terminal(term_event) => match term_event {
                    TermEvent::Key(KeyEvent { code, modifiers }) => match code {
                        KeyCode::Esc => {
                            self.event_queue
                                .sender()
                                .send_with_priority(Event::Close(None));
                        }
                        KeyCode::Char(character) => {
                            if character == 'c' && modifiers.contains(KeyModifiers::CONTROL) {
                                self.event_queue
                                    .sender()
                                    .send_with_priority(Event::Close(None));
                            } else {
                                state.input_write(character);
                            }
                        }
                        KeyCode::Enter => {
                            if let Some(input) = state.reset_input() {
                                let message = if let Err(e) = self.network.send_all(
                                    state.all_user_endpoints(),
                                    NetMessage::UserMessage(input.clone()),
                                ) {
                                    termchat_error_message(stringify_sendall_errors(e))
                                } else {
                                    LogMessage::new(
                                        format!("{} (me)", self.user_name),
                                        MessageType::Content(input.clone()),
                                    )
                                };

                                state.add_message(message);

                                if let Err(parse_error) = self.parse_input(&input, &mut state) {
                                    state.add_message(termchat_error_message(
                                        parse_error.to_string(),
                                    ));
                                }
                            }
                        }
                        KeyCode::Delete => {
                            state.input_remove();
                        }
                        KeyCode::Backspace => {
                            state.input_remove_previous();
                        }
                        KeyCode::Left => {
                            state.input_move_cursor(CursorMovement::Left);
                        }
                        KeyCode::Right => {
                            state.input_move_cursor(CursorMovement::Right);
                        }
                        KeyCode::Home => {
                            state.input_move_cursor(CursorMovement::Start);
                        }
                        KeyCode::End => {
                            state.input_move_cursor(CursorMovement::End);
                        }
                        KeyCode::Up => {
                            state.messages_scroll(ScrollMovement::Up);
                        }
                        KeyCode::Down => {
                            state.messages_scroll(ScrollMovement::Down);
                        }
                        KeyCode::PageUp => {
                            state.messages_scroll(ScrollMovement::Start);
                        }
                        _ => (),
                    },
                    TermEvent::Mouse(_) => (),
                    TermEvent::Resize(_, _) => (),
                },
                Event::Close(e) => {
                    if let Some(error) = e {
                        return Err(error);
                    } else {
                        return Ok(());
                    }
                }
            }
            ui::draw(&mut self.terminal, &state)?;
        }
    }

    fn parse_input(&mut self, input: &str, state: &mut ApplicationState) -> Result<()> {
        use std::io::Read;
        const SEND_COMMAND: &str = "?send";
        const READ_FILENAME_ERROR: &str = "Unable to read file name";

        if input.starts_with(SEND_COMMAND) {
            let path =
                std::path::Path::new(input.split_whitespace().nth(1).ok_or("No file specifed")?);
            let file_name = path
                .file_name()
                .ok_or(READ_FILENAME_ERROR)?
                .to_str()
                .ok_or(READ_FILENAME_ERROR)?
                .to_string();

            let mut file = std::fs::File::open(path)?;
            const BLOCK: usize = 1024;
            let mut data = [0; BLOCK];

            loop {
                match file.read(&mut data) {
                    Ok(bytes_read) => {
                        let data_to_send = data[..bytes_read].to_vec();

                        self.network
                            .send_all(
                                state.all_user_endpoints(),
                                NetMessage::UserData(
                                    file_name.clone(),
                                    Some((data_to_send, bytes_read)),
                                    None,
                                ),
                            )
                            .map_err(stringify_sendall_errors)?;

                        // done
                        if bytes_read == 0 {
                            break;
                        }
                    }
                    Err(e) => {
                        self.network
                            .send_all(
                                state.all_user_endpoints(),
                                NetMessage::UserData(file_name, None, Some(e.to_string())),
                            )
                            .map_err(stringify_sendall_errors)?;
                        return Err(e.into());
                    }
                }
            }
        }
        Ok(())
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        clean_terminal();
    }
}

struct Guard;
impl Drop for Guard {
    fn drop(&mut self) {
        clean_terminal();
    }
}

fn clean_terminal() {
    io::stdout()
        .execute(terminal::LeaveAlternateScreen)
        .expect("Could not leave alternate screen");
    terminal::disable_raw_mode().expect("Could not disable raw mode at exit");
    if std::thread::panicking() {
        eprintln!(
            "termchat paniced, to log the error you can redirect stderror to a file, example: `termchat 2>termchat_log`"
        );
    }
}

fn termchat_error_message(e: String) -> LogMessage {
    LogMessage::new(String::from("termchat: "), MessageType::Error(e))
}

fn stringify_sendall_errors(e: Vec<(message_io::network::Endpoint, io::Error)>) -> String {
    let mut out = String::new();
    for (endpoint, error) in e {
        let msg = format!("Failed to connect to {}, error: {}", endpoint, error);
        out.push_str(&msg);
        out.push('\n');
    }
    // remove last new line
    if !out.is_empty() {
        out.pop();
    }
    out
}
