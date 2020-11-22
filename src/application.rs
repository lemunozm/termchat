use super::state::{
    State, CursorMovement, LogMessage, MessageType, ScrollMovement, TermchatMessageType,
};
use super::terminal_events::TerminalEventCollector;
use super::ui::{self};
use crate::util::{stringify_sendall_errors, termchat_message, Error, Result};

use crossterm::event::{Event as TermEvent, KeyCode, KeyEvent, KeyModifiers};
use crossterm::{
    terminal::{self},
    ExecutableCommand,
};

use tui::backend::CrosstermBackend;
use tui::Terminal;

use message_io::events::EventQueue;
use message_io::network::{NetEvent, NetworkManager, Endpoint};

use serde::{Deserialize, Serialize};

use std::io::{self, Stdout};
use std::net::{SocketAddrV4};

mod commands;
mod read_event;

use read_event::{read_file, Chunk, ReadFile};

#[derive(Serialize, Deserialize)]
enum NetMessage {
    HelloLan(String, u16), // user_name, server_port
    HelloUser(String),     // user_name
    UserMessage(String),   // content
    UserData(String, Option<(Vec<u8>, usize)>, Option<String>), // file_name, Option<data, bytes_read>, Option<Error>
}

enum Event {
    Network(NetEvent<NetMessage>),
    Terminal(TermEvent),
    ReadFile(Result<Chunk>),
    // Close event with an optional error in case of failure
    // Close(None) means no error happened
    Close(Option<Error>),
}

pub struct Config {
    pub discovery_addr: SocketAddrV4,
    pub tcp_server_port: u16,
    pub user_name: String,
}

pub struct Application<'a> {
    config: &'a Config,
    state: State,
    network: NetworkManager,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    read_file_ev: ReadFile,
    _terminal_events: TerminalEventCollector,
    event_queue: EventQueue<Event>,
}

impl<'a> Application<'a> {
    pub fn new(config: &'a Config) -> Result<Application<'a>> {
        let mut event_queue = EventQueue::new();

        let sender = event_queue.sender().clone(); // Collect network events
        let network = NetworkManager::new(move |net_event| {
            sender.send(Event::Network(net_event))
        });

        let sender = event_queue.sender().clone(); // Collect terminal events
        let _terminal_events = TerminalEventCollector::new(move |term_event| match term_event {
            Ok(event) => sender.send(Event::Terminal(event)),
            Err(e) => sender.send(Event::Close(Some(e))),
        })?;

        let sender = event_queue.sender().clone(); // Collect read_file events
        let read_file_ev = ReadFile::new(Box::new(move |file, file_name, file_size, id| {
            let chunk = read_file(file, file_name, file_size, id);
            sender.send(Event::ReadFile(chunk));
        }));

        let terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

        // Guard to make sure to cleanup if a failure happens in the next lines
        let _g = Guard;

        terminal::enable_raw_mode()?;
        io::stdout().execute(terminal::EnterAlternateScreen)?;

        std::mem::forget(_g);

        Ok(Application {
            config,
            state: State::new(),
            network,
            terminal,
            read_file_ev,
            // Stored because we want its internal thread functionality until the Application was dropped
            _terminal_events,
            event_queue,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        ui::draw(&mut self.terminal, &self.state)?;

        let server_addr = ("0.0.0.0", self.config.tcp_server_port);
        let (_, server_addr) = self.network.listen_tcp(server_addr)?;
        self.network.listen_udp_multicast(self.config.discovery_addr)?;

        let discovery_endpoint = self.network.connect_udp(self.config.discovery_addr)?;
        let message = NetMessage::HelloLan(self.config.user_name.clone(), server_addr.port());
        self.network.send(discovery_endpoint, message)?;

        loop {
            match self.event_queue.receive() {
                Event::ReadFile(chunk) => {
                    let try_send = || -> Result<()> {
                        let Chunk { file, id, file_name, data, bytes_read, file_size } = chunk?;

                        self.network
                            .send_all(
                                self.state.all_user_endpoints(),
                                NetMessage::UserData(
                                    file_name.clone(),
                                    Some((data, bytes_read)),
                                    None,
                                ),
                            )
                            .map_err(stringify_sendall_errors)?;

                        if bytes_read == 0 {
                            self.state.progress_stop(id);
                        }
                        else {
                            self.state.progress_pulse(id, file_size, bytes_read);
                            let chunk = read_file(file, file_name, file_size, id);
                            self.event_queue.sender().send(Event::ReadFile(chunk));
                        }
                        Ok(())
                    };

                    if let Err(e) = try_send() {
                        // we dont have the file_name here
                        // we'll just stop the last progress
                        self.state.progress_stop_last();
                        let msg = format!("Error sending file. error: {}", e);
                        self.state.add_message(termchat_message(msg, TermchatMessageType::Error));
                    }
                }
                Event::Network(net_event) => match net_event {
                    NetEvent::Message(endpoint, message) =>
                        self.process_network_message(endpoint, message),
                    NetEvent::AddedEndpoint(_) => (),
                    NetEvent::RemovedEndpoint(endpoint) =>
                        self.state.disconnected_user(endpoint)
                },
                Event::Terminal(term_event) =>
                    self.process_terminal_event(term_event),
                Event::Close(error) => {
                    return match error {
                        Some(error) => Err(error),
                        None => Ok(())
                    }
                }
            }
            ui::draw(&mut self.terminal, &self.state)?;
        }
    }

    fn process_network_message(&mut self, endpoint: Endpoint, message: NetMessage) {
        match message {
            // by udp (multicast):
            NetMessage::HelloLan(user, server_port) => {
                let server_addr = (endpoint.addr().ip(), server_port);
                if user != self.config.user_name {
                    let mut try_connect = || -> Result<()> {
                        let user_endpoint = self.network.connect_tcp(server_addr)?;
                        let message = NetMessage::HelloUser(self.config.user_name.clone());
                        self.network.send(user_endpoint, message)?;
                        self.state.connected_user(user_endpoint, &user);
                        Ok(())
                    };
                    if let Err(e) = try_connect() {
                        let message =
                            termchat_message(e.to_string(), TermchatMessageType::Error);
                        self.state.add_message(message);
                    }
                }
            }
            // by tcp:
            NetMessage::HelloUser(user) => {
                self.state.connected_user(endpoint, &user);
            }
            NetMessage::UserMessage(content) => {
                if let Some(user) = self.state.user_name(endpoint) {
                    let message =
                        LogMessage::new(user.into(), MessageType::Content(content));
                    self.state.add_message(message);
                }
            }
            NetMessage::UserData(file_name, maybe_data, maybe_error) => {
                use std::io::Write;
                if self.state.user_name(endpoint).is_some() {
                    // safe unwrap due to check
                    let user = self.state.user_name(endpoint).unwrap().to_owned();

                    let try_write = || -> Result<()> {
                        if let Some(error) = maybe_error {
                            return Err(format!(
                                "{} encountred an error while sending {}, error: {}",
                                user, file_name, error
                            )
                            .into())
                        }
                        // if the error is none we know that maybe_data is some
                        let (data, bytes_read) = maybe_data.unwrap();

                        //done
                        if bytes_read == 0 {
                            let msg = format!(
                                "Successfully received file {} from user {} !",
                                file_name, user
                            );
                            let msg = termchat_message(
                                msg,
                                TermchatMessageType::Notification,
                            );
                            self.state.add_message(msg);
                            return Ok(())
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
                        let message = format!(
                            "termchat: Failed to write data sent from user: {}",
                            user
                        );
                        self.state.add_message(termchat_message(
                            message,
                            TermchatMessageType::Error,
                        ));
                        self.state.add_message(termchat_message(
                            e.to_string(),
                            TermchatMessageType::Error,
                        ));
                    }
                }
            }
        }
    }

    fn process_terminal_event(&mut self, term_event: TermEvent) {
        match term_event {
            TermEvent::Mouse(_) => (),
            TermEvent::Resize(_, _) => (),
            TermEvent::Key(KeyEvent { code, modifiers }) => match code {
                KeyCode::Esc => {
                    self.event_queue.sender().send_with_priority(Event::Close(None));
                }
                KeyCode::Char(character) => {
                    if character == 'c' && modifiers.contains(KeyModifiers::CONTROL) {
                        self.event_queue.sender().send_with_priority(Event::Close(None));
                    }
                    else {
                        self.state.input_write(character);
                    }
                }
                KeyCode::Enter => {
                    if let Some(input) = self.state.reset_input() {
                        let message = if let Err(e) = self.network.send_all(
                            self.state.all_user_endpoints(),
                            NetMessage::UserMessage(input.clone()),
                        ) {
                            termchat_message(
                                stringify_sendall_errors(e),
                                TermchatMessageType::Error,
                            )
                        }
                        else {
                            LogMessage::new(
                                format!("{} (me)", self.config.user_name),
                                MessageType::Content(input.clone()),
                            )
                        };

                        self.state.add_message(message);

                        if let Err(parse_error) = self.parse_input(&input) {
                            self.state.add_message(termchat_message(
                                parse_error.to_string(),
                                TermchatMessageType::Error,
                            ));
                        }
                    }
                }
                KeyCode::Delete => {
                    self.state.input_remove();
                }
                KeyCode::Backspace => {
                    self.state.input_remove_previous();
                }
                KeyCode::Left => {
                    self.state.input_move_cursor(CursorMovement::Left);
                }
                KeyCode::Right => {
                    self.state.input_move_cursor(CursorMovement::Right);
                }
                KeyCode::Home => {
                    self.state.input_move_cursor(CursorMovement::Start);
                }
                KeyCode::End => {
                    self.state.input_move_cursor(CursorMovement::End);
                }
                KeyCode::Up => {
                    self.state.messages_scroll(ScrollMovement::Up);
                }
                KeyCode::Down => {
                    self.state.messages_scroll(ScrollMovement::Down);
                }
                KeyCode::PageUp => {
                    self.state.messages_scroll(ScrollMovement::Start);
                }
                _ => (),
            },
        }
    }

    fn process_command_event(&mut self) {

    }
}

impl Drop for Application<'_> {
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
    io::stdout().execute(terminal::LeaveAlternateScreen).expect("Could not execute to stdout");
    terminal::disable_raw_mode().expect("Terminal doesn't support to disable raw mode");
    if std::thread::panicking() {
        eprintln!(
            "{}, example: {}",
            "termchat paniced, to log the error you can redirect stderror to a file",
            "`termchat 2> termchat_log`"
        );
    }
}
