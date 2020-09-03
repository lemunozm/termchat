use super::terminal_events::{TerminalEventCollector};
use super::state::{ApplicationState, UserMessage, CursorMovement, ScrollMovement};
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
                        KeyCode::Char(character) => {
                            if character == 'c' && modifiers.contains(KeyModifiers::CONTROL) {
                                self.event_queue.sender().send_with_priority(Event::Close);
                            }
                            else {
                                state.input_write(character);
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
                        KeyCode::Enter => {
                            if let Some(message) = state.reset_input() {
                                state.add_message(UserMessage {
                                    date: Local::now(),
                                    user: format!("{} (me)", self.name),
                                    msg: message,
                                });
                            }
                        },
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

