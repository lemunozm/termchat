use super::state::{ApplicationState, CursorMovement, LogMessage, MessageType, ScrollMovement};
use super::terminal_events::TerminalEventCollector;
use super::ui::{self};
use crate::util::Result;

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
}

enum Event {
    Network(NetEvent<NetMessage>),
    Terminal(TermEvent),
    // Close event with optionaly an error
    // Close(None) means no error happened
    Close(Option<String>),
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
        match Self::try_new(discovery_addr, tcp_server_port, user_name) {
            Ok(app) => Ok(app),
            Err(e) => {
                clean_terminal();
                Err(e)
            }
        }
    }
    fn try_new(
        discovery_addr: SocketAddr,
        tcp_server_port: u16,
        user_name: &str,
    ) -> Result<Application> {
        let mut event_queue = EventQueue::new();

        let sender = event_queue.sender().clone(); // Collect network events
        let network = NetworkManager::new(move |net_event| sender.send(Event::Network(net_event)));

        let sender = event_queue.sender().clone(); // Collect terminal events
        let _terminal_events = TerminalEventCollector::new(move |term_event| match term_event {
            Ok(event) => sender.send(Event::Terminal(event)),
            Err(e) => sender.send(Event::Close(Some(e.to_string()))),
        })?;

        terminal::enable_raw_mode()?;
        io::stdout().execute(terminal::EnterAlternateScreen)?;
        let terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

        let tcp_server_addr = ([0, 0, 0, 0], tcp_server_port).into();

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

    pub fn run(&mut self) {
        if let Err(e) = self.try_run() {
            clean_terminal();
            eprintln!("termchat crashed with error: {}", e);
        } else {
            clean_terminal();
        }
    }

    fn try_run(&mut self) -> Result<()> {
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
                            // Note: can this unwrap actually fail?
                            let user = "???".to_string();
                            let user = state.user_name(endpoint).unwrap_or(&user);

                            let message =
                                LogMessage::new(user.into(), MessageType::Content(content));
                            state.add_message(message);
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
                                    LogMessage::new(
                                        String::from("termchat :"),
                                        MessageType::Error(format_errors(e)),
                                    )
                                } else {
                                    LogMessage::new(
                                        format!("{} (me)", self.user_name),
                                        MessageType::Content(input),
                                    )
                                };

                                state.add_message(message);
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
                        return Err(error.into());
                    } else {
                        return Ok(());
                    }
                }
            }
            ui::draw(&mut self.terminal, &state)?;
        }
    }
}

fn clean_terminal() {
    io::stdout()
        .execute(terminal::LeaveAlternateScreen)
        .expect("Could not leave alternate screen");
    terminal::disable_raw_mode().expect("Could not disable raw mode at exit");
}

fn format_errors(e: Vec<(message_io::network::Endpoint, io::Error)>) -> String {
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
