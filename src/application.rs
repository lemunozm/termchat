use super::terminal_events::{TerminalEventCollector};
use super::state::{ApplicationState, LogMessage, MessageType, CursorMovement, ScrollMovement};
use super::ui::{self};

use crossterm::{ExecutableCommand, terminal::{self}};
use crossterm::event::{Event as TermEvent, KeyEvent, KeyCode, KeyModifiers};

use tui::{Terminal};
use tui::backend::{CrosstermBackend};

use message_io::events::{EventQueue};
use message_io::network::{NetworkManager, NetEvent};

use serde::{Serialize, Deserialize};

use std::net::{SocketAddr};
use std::io::{self, Stdout};

#[derive(Serialize, Deserialize)]
enum NetMessage {
    HelloLan(String, SocketAddr), // user_name, server_addr
    HelloUser(String), // user_name
    UserMessage(String), // content
}

enum Event {
    Network(NetEvent<NetMessage>),
    Terminal(TermEvent),
    Close,
}

pub struct Application {
    event_queue: EventQueue<Event>,
    network: NetworkManager,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    _terminal_events: TerminalEventCollector,
    discovery_addr: SocketAddr,
    user_name: String,
}

impl Application {
    pub fn new(discovery_addr: SocketAddr, user_name: &str) -> io::Result<Application> {
        let mut event_queue = EventQueue::new();

        let sender = event_queue.sender().clone(); // Collect network events
        let network = NetworkManager::new(move |net_event| sender.send(Event::Network(net_event)));

        let sender = event_queue.sender().clone(); // Collect terminal events
        let _terminal_events = TerminalEventCollector::new(move |term_event| sender.send(Event::Terminal(term_event)));

        terminal::enable_raw_mode().unwrap();
        io::stdout().execute(terminal::EnterAlternateScreen).unwrap();
        let terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

        Ok(Application {
            event_queue,
            network,
            terminal,
            // Stored because we want its internal thread functionality until the Application was dropped
            _terminal_events,
            discovery_addr,
            user_name: user_name.into(),
        })
    }

    pub fn run(&mut self) {
        let mut state = ApplicationState::new();
        ui::draw(&mut self.terminal, &state);

        let (_, server_addr) = self.network.listen_tcp("0.0.0.0:0").unwrap();
        let discovery_endpoint = self.network.connect_udp(self.discovery_addr).unwrap();
        self.network.send(discovery_endpoint, NetMessage::HelloLan(self.user_name.clone(), server_addr)).unwrap();
        self.network.listen_udp_multicast(self.discovery_addr).unwrap();

        loop {
            match self.event_queue.receive() {
                Event::Network(net_event) => match net_event {
                    NetEvent::Message(endpoint, message) => match message {
                        // by udp (multicast):
                        NetMessage::HelloLan(user, server_addr) => {
                            if user != self.user_name {
                                let user_endpoint = self.network.connect_tcp(server_addr).unwrap();
                                self.network.send(user_endpoint, NetMessage::HelloUser(self.user_name.clone())).unwrap();
                                state.connected_user(user_endpoint, &user);
                            }
                        },
                        // by tcp:
                        NetMessage::HelloUser(user) => {
                            state.connected_user(endpoint, &user);
                        }
                        NetMessage::UserMessage(content) => {
                            let user = state.user_name(endpoint).unwrap();
                            let message = LogMessage::new(user.into(), MessageType::Content(content));
                            state.add_message(message);
                        }
                    },
                    NetEvent::AddedEndpoint(_) => (),
                    NetEvent::RemovedEndpoint(endpoint) => {
                        state.disconnected_user(endpoint);
                    },
                },
                Event::Terminal(term_event) => match term_event {
                    TermEvent::Key(KeyEvent{code, modifiers}) => match code {
                        KeyCode::Esc => {
                            self.event_queue.sender().send_with_priority(Event::Close);
                        },
                        KeyCode::Char(character) => {
                            if character == 'c' && modifiers.contains(KeyModifiers::CONTROL) {
                                self.event_queue.sender().send_with_priority(Event::Close);
                            }
                            else {
                                state.input_write(character);
                            }
                        },
                        KeyCode::Enter => {
                            if let Some(input) = state.reset_input() {
                                let message = LogMessage::new(format!("{} (me)", state.all_user_endpoints().count()), MessageType::Content(input.clone()));
                                self.network.send_all(state.all_user_endpoints(), NetMessage::UserMessage(input)).unwrap();
                                state.add_message(message);
                            }
                        },
                        KeyCode::Delete => {
                            state.input_remove();
                        },
                        KeyCode::Backspace => {
                            state.input_remove_previous();
                        },
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
                        },
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
                }
                Event::Close => break,
            }
            ui::draw(&mut self.terminal, &state);
        }
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        io::stdout().execute(terminal::LeaveAlternateScreen).unwrap();
        terminal::disable_raw_mode().unwrap()
    }
}

