use super::state::{
    ApplicationState, CursorMovement, LogMessage, MessageType, ScrollMovement, TermchatMessageType,
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
use message_io::network::{NetEvent, NetworkManager};

use serde::{Deserialize, Serialize};

use std::io::{self, Stdout};
use std::net::SocketAddr;

mod commands;
use crate::read_event::{Chunk, ReadFile};

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

pub struct Application {
    event_queue: EventQueue<Event>,
    network: NetworkManager,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    read_file_ev: ReadFile,
    _terminal_events: TerminalEventCollector,
    discovery_addr: SocketAddr,
    tcp_server_addr: SocketAddr,
    user_name: String,
    // id is used to identify the progress of sent files
    id: usize,
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
        let network = NetworkManager::new(move |net_event| {
            sender.send_with_priority(Event::Network(net_event))
        });

        let sender = event_queue.sender().clone(); // Collect terminal events
        let _terminal_events = TerminalEventCollector::new(move |term_event| match term_event {
            Ok(event) => sender.send_with_priority(Event::Terminal(event)),
            Err(e) => sender.send(Event::Close(Some(e))),
        })?;

        let sender = event_queue.sender().clone(); // Collect read_file events
        let read_file_ev =
            ReadFile::new(Box::new(move |chunk| sender.send(Event::ReadFile(chunk))));

        terminal::enable_raw_mode()?;
        io::stdout().execute(terminal::EnterAlternateScreen)?;
        let terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

        let tcp_server_addr = ([0, 0, 0, 0], tcp_server_port).into();

        std::mem::forget(_g);

        Ok(Application {
            event_queue,
            network,
            terminal,
            read_file_ev,
            // Stored because we want its internal thread functionality until the Application was dropped
            _terminal_events,
            discovery_addr,
            tcp_server_addr,
            user_name: user_name.into(),
            id: 0,
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
                Event::ReadFile(chunk) => {
                    let try_send = || -> Result<()> {
                        let Chunk {
                            id,
                            file_name,
                            data,
                            bytes_read,
                            file_size,
                        } = chunk?;

                        self.network
                            .send_all(
                                state.all_user_endpoints(),
                                NetMessage::UserData(file_name, Some((data, bytes_read)), None),
                            )
                            .map_err(stringify_sendall_errors)?;

                        if bytes_read == 0 {
                            state.progress_stop(id);
                        } else {
                            state.progress_pulse(id, file_size, bytes_read);
                        }
                        Ok(())
                    };

                    if let Err(e) = try_send() {
                        // we dont have the file_name here
                        // we'll just stop the last progress
                        state.progress_stop_last();
                        let msg = format!("Error sending file. error: {}", e);
                        state.add_message(termchat_message(msg, TermchatMessageType::Error));
                    }
                }
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
                                    let message =
                                        termchat_message(e.to_string(), TermchatMessageType::Error);
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
                                        let msg = format!(
                                            "Successfully received file {} from user {} !",
                                            file_name, user
                                        );
                                        let msg = termchat_message(
                                            msg,
                                            TermchatMessageType::Notification,
                                        );
                                        state.add_message(msg);
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
                                    let message = format!(
                                        "termchat: Failed to write data sent from user: {}",
                                        user
                                    );
                                    state.add_message(termchat_message(
                                        message,
                                        TermchatMessageType::Error,
                                    ));
                                    state.add_message(termchat_message(
                                        e.to_string(),
                                        TermchatMessageType::Error,
                                    ));
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
                                    termchat_message(
                                        stringify_sendall_errors(e),
                                        TermchatMessageType::Error,
                                    )
                                } else {
                                    LogMessage::new(
                                        format!("{} (me)", self.user_name),
                                        MessageType::Content(input.clone()),
                                    )
                                };

                                state.add_message(message);

                                if let Err(parse_error) = self.parse_input(&input, &mut state) {
                                    state.add_message(termchat_message(
                                        parse_error.to_string(),
                                        TermchatMessageType::Error,
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
