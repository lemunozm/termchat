use super::terminal_events::{TerminalEventCollector};
use super::state::{ApplicationState, UserMessage};
use super::ui::{self};

use crossterm::{ExecutableCommand, terminal::{self}};
use crossterm::event::{Event as TermEvent, KeyEvent, KeyCode, KeyModifiers};

use tui::{Terminal};
use tui::backend::{CrosstermBackend};

use message_io::events::{EventQueue};
use message_io::network::{NetworkManager, NetEvent};

use serde::{Serialize, Deserialize};

use chrono::{Local};

use std::net::{SocketAddr};
use std::io::{self, Stdout};

#[derive(Serialize)]
enum OutputMessage {
    ParticipantInfo(String),
}

#[derive(Deserialize)]
enum InputMessage {
    HelloLan,
}

enum Event {
    Network(NetEvent<InputMessage>),
    Terminal(TermEvent),
    Close,
}

pub struct Application {
    event_queue: EventQueue<Event>,
    network: NetworkManager,
    terminal: Terminal<CrosstermBackend<Stdout>>,
    _terminal_events: TerminalEventCollector,
    name: String,
}

impl Application {
    pub fn new(discovery_addr: SocketAddr, name: &str) -> io::Result<Application> {
        let mut event_queue = EventQueue::new();

        let sender = event_queue.sender().clone(); // Collect network events
        let mut network = NetworkManager::new(move |net_event| sender.send(Event::Network(net_event)));

        let sender = event_queue.sender().clone(); // Collect terminal events
        let _terminal_events = TerminalEventCollector::new(move |term_event| sender.send(Event::Terminal(term_event)));

        network.listen_udp_multicast(discovery_addr).unwrap();

        terminal::enable_raw_mode().unwrap();
        io::stdout().execute(terminal::EnterAlternateScreen).unwrap();
        let terminal = Terminal::new(CrosstermBackend::new(io::stdout()))?;

        Ok(Application {
            event_queue,
            network,
            terminal,
            // Stored because we want its internal thread functionality until the Application was dropped
            _terminal_events,
            name: name.into(),
        })
    }

    pub fn run(&mut self) {
        let mut state = ApplicationState::new();
        ui::draw(&mut self.terminal, &state);
        loop {
            match self.event_queue.receive() {
                Event::Network(net_event) => match net_event {
                    NetEvent::Message(_, message) => match message {
                        InputMessage::HelloLan => { },
                    },
                    NetEvent::AddedEndpoint(_) => (),
                    NetEvent::RemovedEndpoint(_) => (),
                },
                Event::Terminal(term_event) => match term_event {
                    TermEvent::Key(KeyEvent{code, modifiers}) => match code {
                        KeyCode::Esc => {
                            self.event_queue.sender().send_with_priority(Event::Close);
                        },
                        KeyCode::Char(c) => {
                            if c == 'c' && modifiers.contains(KeyModifiers::CONTROL) {
                                self.event_queue.sender().send_with_priority(Event::Close);
                            }
                            else {
                                state.input.insert(state.input_cursor, c);
                                state.input_cursor += 1;
                            }
                        },
                        KeyCode::Backspace => {
                            if state.input_cursor > 0 {
                                state.input_cursor -= 1;
                                state.input.remove(state.input_cursor);
                            }
                        },
                        KeyCode::Left => {
                            if state.input_cursor > 0 {
                                state.input_cursor -= 1;
                            }
                        }
                        KeyCode::Right => {
                            if state.input_cursor < state.input.len() {
                                state.input_cursor += 1;
                            }
                        }
                        KeyCode::Home => {
                            state.input_cursor = 0;
                        }
                        KeyCode::End => {
                            state.input_cursor = state.input.len();
                        }
                        KeyCode::Enter => {
                            if state.input.len() > 0 {
                                state.messages.push(UserMessage {
                                    user: format!("{} (me)", self.name),
                                    data: state.input.drain(..).collect(),
                                    date: Local::now(),
                                });
                                state.input_cursor = 0;
                            }
                        },
                        KeyCode::Up => {
                            if state.scroll_messages_view > 0 {
                                state.scroll_messages_view -= 1;
                            }
                        },
                        KeyCode::Down => {
                            state.scroll_messages_view += 1;
                        }
                        KeyCode::PageUp => {
                            state.scroll_messages_view = 0;
                        }
                        KeyCode::PageDown => {
                            //TODO: when tui-rs support a kind of 'max-scrolling' for paragraph
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

