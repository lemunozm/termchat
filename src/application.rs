use super::terminal_events::{TerminalEventCollector};

use crossterm::{ExecutableCommand, terminal::{self}};
use crossterm::event::{Event as TermEvent, KeyEvent, KeyCode, KeyModifiers};

use tui::Terminal;
use tui::backend::CrosstermBackend;
use tui::widgets::{Widget, Block, Borders, List, ListItem, ListState, Paragraph};
use tui::layout::{Layout, Constraint, Direction, Rect, Alignment};
use tui::style::{Style, Modifier, Color};

use message_io::events::{EventQueue};
use message_io::network::{NetworkManager, NetEvent};

use serde::{Serialize, Deserialize};

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
}

impl Application {
    pub fn new(discovery_addr: SocketAddr) -> io::Result<Application> {
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
        })
    }

    pub fn run(&mut self) {
        self.render();
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
                        KeyCode::Char('c') => {
                            if modifiers.contains(KeyModifiers::CONTROL) {
                                self.event_queue.sender().send_with_priority(Event::Close);
                            }
                        }
                        _ => (),
                    },
                    TermEvent::Mouse(_) => (),
                    TermEvent::Resize(_, _) => (),
                }
                Event::Close => break,
            }
            self.render();
        }
    }

    fn render(&mut self) {
        self.terminal.draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Min(0),
                        Constraint::Length(6)
                    ].as_ref()
                )
                .split(frame.size());

            let items = vec![
                ListItem::new("Item 1"),
                ListItem::new("Item 2"),
                ListItem::new("Item 3")
            ];

            let conversation_panel = List::new(items)
                .block(Block::default().title("LAN Room").borders(Borders::ALL))
                .style(Style::default().fg(Color::White));

            frame.render_widget(conversation_panel, chunks[0]);

            let writing_panel = Paragraph::new("This is an example text")
                .block(Block::default().title("Your message").borders(Borders::ALL))
                .style(Style::default().fg(Color::White))
                .alignment(Alignment::Left);

            frame.render_widget(writing_panel, chunks[1]);
        }).unwrap()
    }
}

impl Drop for Application {
    fn drop(&mut self) {
        io::stdout().execute(terminal::LeaveAlternateScreen).unwrap();
        terminal::disable_raw_mode().unwrap()
    }
}

