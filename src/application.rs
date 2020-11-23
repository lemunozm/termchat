use super::state::{State, CursorMovement, ChatMessage, MessageType, ScrollMovement};
use crate::terminal_events::{TerminalEventCollector};
use crate::renderer::{Renderer};
use crate::action::{Action, Processing};
use crate::commands::{CommandManager};
use crate::util::{self, Error, Result};

use crossterm::event::{Event as TermEvent, KeyCode, KeyEvent, KeyModifiers};

use message_io::events::{EventQueue};
use message_io::network::{NetEvent, NetworkManager, Endpoint};

use serde::{Deserialize, Serialize};

use std::net::{SocketAddrV4};

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
    Action(Box<dyn Action>),
    //ReadFile(Result<Chunk>),
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
    commands: CommandManager,
    //read_file_ev: ReadFile,
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
        let _terminal_events = TerminalEventCollector::new(move |term_event| {
            match term_event {
                Ok(event) => sender.send(Event::Terminal(event)),
                Err(e) => sender.send(Event::Close(Some(e))),
            }
        })?;

        Ok(Application {
            config,
            state: State::new(),
            network,
            commands: CommandManager::default(),
            // Stored because we want its internal thread running until the Application was dropped
            _terminal_events,
            event_queue,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        let mut renderer = Renderer::new()?;
        renderer.render(&self.state)?;

        let server_addr = ("0.0.0.0", self.config.tcp_server_port);
        let (_, server_addr) = self.network.listen_tcp(server_addr)?;
        self.network.listen_udp_multicast(self.config.discovery_addr)?;

        let discovery_endpoint = self.network.connect_udp(self.config.discovery_addr)?;
        let message = NetMessage::HelloLan(self.config.user_name.clone(), server_addr.port());
        self.network.send(discovery_endpoint, message)?;

        loop {
            match self.event_queue.receive() {
                Event::Network(net_event) => match net_event {
                    NetEvent::Message(endpoint, message) => {
                        self.process_network_message(endpoint, message);
                    }
                    NetEvent::AddedEndpoint(_) => (),
                    NetEvent::RemovedEndpoint(endpoint) => {
                        self.state.disconnected_user(endpoint)
                    }
                },
                Event::Terminal(term_event) => {
                    self.process_terminal_event(term_event);
                }
                Event::Action(action) => {
                    self.process_action(action);
                }
                Event::Close(error) => {
                    return match error {
                        Some(error) => Err(error),
                        None => Ok(())
                    }
                }
            }
            renderer.render(&self.state)?;
        }
        //Renderer is destroyed here and the terminal is recovered
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
                        self.state.add_system_error_message(e.to_string());
                    }
                }
            }
            // by tcp:
            NetMessage::HelloUser(user) => {
                self.state.connected_user(endpoint, &user);
            }
            NetMessage::UserMessage(content) => {
                if let Some(user) = self.state.user_name(endpoint) {
                    let message = ChatMessage::new(user.into(), MessageType::Text(content));
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
                            self.state.add_system_info_message(msg);
                            return Ok(())
                        }

                        let user_path = std::env::temp_dir().join("termchat").join(&user);

                        match std::fs::create_dir_all(&user_path) {
                            Ok(_) => (),
                            Err(ref err) if err.kind() == std::io::ErrorKind::Interrupted => (),
                            Err(e) => Err(e)?,
                        }

                        let file_path = user_path.join(file_name);

                        std::fs::OpenOptions::new()
                            .create(true)
                            .append(true)
                            .open(file_path)?
                            .write_all(&data)?;

                        Ok(())
                    };

                    if let Err(error) = try_write() {
                        let message = format!(
                            "termchat: Failed to write data sent from user: {}",
                            user
                        );
                        self.state.add_system_error_message(message);
                        self.state.add_system_error_message(error.to_string());
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
                        let should_send = match self.commands.find_command_action(&input) {
                            Some(Ok(action)) => self.process_action(action),
                            Some(Err(e)) => {
                                self.state.add_system_error_message(e.to_string());
                                false
                            }
                            None => true,
                        };

                        if should_send {
                            let result = self.network.send_all(
                                self.state.all_user_endpoints(),
                                NetMessage::UserMessage(input.clone())
                            );

                            //TODO: Should print the Ok version if some endpoint is connected
                            match result {
                                Ok(_) => {
                                    let message = ChatMessage::new(
                                        format!("{} (me)", self.config.user_name),
                                        MessageType::Text(input.clone()),
                                    );
                                    self.state.add_message(message);
                                },
                                Err(e) => {
                                    let errors = util::stringify_sendall_errors(e);
                                    self.state.add_system_error_message(errors);
                                }
                            }
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

    fn process_action(&mut self, mut action: Box<dyn Action>) -> bool {
        match action.process(&mut self.state, &mut self.network) {
            Ok(Processing::Completed) => true,
            Ok(Processing::Partial) => {
                self.event_queue.sender().send(Event::Action(action));
                true
            }
            Err(error) => {
                self.state.add_system_error_message(error.to_string());
                false
            }
        }
    }
}
