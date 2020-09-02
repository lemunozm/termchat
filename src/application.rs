use super::terminal_events::{TerminalEventCollector};

use crossterm::{ExecutableCommand, terminal::{self}};
use crossterm::event::{Event as TermEvent, KeyEvent, KeyCode, KeyModifiers};

use tui::{Terminal, Frame};
use tui::backend::CrosstermBackend;
use tui::widgets::{Widget, Block, Borders, List, ListItem, ListState, Paragraph, Wrap};
use tui::layout::{Layout, Constraint, Direction, Rect, Alignment};
use tui::style::{Style, Modifier, Color};
use tui::text::{Span, Spans};

use message_io::events::{EventQueue};
use message_io::network::{NetworkManager, NetEvent};

use serde::{Serialize, Deserialize};

use chrono::{DateTime, Local};

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

struct UserMessage {
   user: String,
   data: String,
   date: DateTime<Local>,
}

struct ApplicationState {
    messages: Vec<UserMessage>,
    input: String,
    input_cursor: usize,
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
            // Stored because we want its internal thread functionality until the Application drop
            _terminal_events,
            name: name.into(),
        })
    }

    pub fn run(&mut self) {
        let mut state = ApplicationState {
            messages: Vec::new(),
            input: String::new(),
            input_cursor: 0,
        };

        self.render(&state);
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
                            state.messages.push(UserMessage {
                                user: format!("{} (me)", self.name),
                                data: state.input.drain(..).collect(),
                                date: Local::now(),
                            });
                            state.input_cursor = 0;
                        },
                        _ => (),
                    },
                    TermEvent::Mouse(_) => (),
                    TermEvent::Resize(_, _) => (),
                }
                Event::Close => break,
            }
            self.render(&state);
        }
    }

    fn render(&mut self, state: &ApplicationState) {
        self.terminal.draw(&mut |frame: &mut Frame<CrosstermBackend<Stdout>>| {
            let messages = state.messages
                .iter()
                .map(|message| {
                    let text = vec![
                        Spans::from(vec![
                            Span::styled(message.date.format("%H:%M:%S ").to_string(), Style::default().fg(Color::DarkGray)),
                            Span::styled(&message.user, Style::default().fg(Color::Green)),
                            Span::styled(": ", Style::default().fg(Color::Green)),
                            Span::raw(&message.data),
                        ])
                    ];
                    ListItem::new(text)
                })
                .collect::<Vec<_>>();

            let messages_panel = List::new(messages)
                .block(Block::default().title("LAN Room").borders(Borders::ALL))
                .style(Style::default().fg(Color::White));

            let input_panel = Paragraph::new(state.input.as_str())
                .block(Block::default().title("Your message").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Left)
                .wrap(Wrap { trim: true });

            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints(
                    [
                        Constraint::Min(0),
                        Constraint::Length(6)
                    ].as_ref()
                )
                .split(frame.size());

            frame.render_widget(messages_panel, chunks[0]);
            frame.render_widget(input_panel, chunks[1]);

            let valid_panel_width = (chunks[1].width - 2) as usize;
            frame.set_cursor(
                chunks[1].x + 1 + (state.input_cursor % valid_panel_width) as u16,
                chunks[1].y + 1 + (state.input_cursor / valid_panel_width) as u16,
            )

        }).unwrap()
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        io::stdout().execute(terminal::LeaveAlternateScreen).unwrap();
        terminal::disable_raw_mode().unwrap()
    }
}

