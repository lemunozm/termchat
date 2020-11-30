use super::state::{ProgressState, State, MessageType, SystemMessageType};
use super::commands::{CommandManager};
use super::util::{split_each};

use tui::backend::CrosstermBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, Paragraph, Wrap};
use tui::{Frame};

use std::io::Write;

pub fn draw(frame: &mut Frame<CrosstermBackend<impl Write>>, state: &State, chunk: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(0), Constraint::Length(6)].as_ref())
        .split(chunk);

    draw_messages_panel(frame, state, chunks[0]);
    draw_input_panel(frame, state, chunks[1]);
}

fn draw_messages_panel(
    frame: &mut Frame<CrosstermBackend<impl Write>>,
    state: &State,
    chunk: Rect,
)
{
    const MESSAGE_COLORS: [Color; 4] = [Color::Blue, Color::Yellow, Color::Cyan, Color::Magenta];

    let messages = state
        .messages()
        .iter()
        .rev()
        .map(|message| {
            let color = if let Some(id) = state.users_id().get(&message.user) {
                MESSAGE_COLORS[id % MESSAGE_COLORS.len()]
            }
            else {
                Color::Green //because is a message of the own user
            };
            let date = message.date.format("%H:%M:%S ").to_string();
            match &message.message_type {
                MessageType::Connection => Spans::from(vec![
                    Span::styled(date, Style::default().fg(Color::DarkGray)),
                    Span::styled(&message.user, Style::default().fg(color)),
                    Span::styled(" is online", Style::default().fg(color)),
                ]),
                MessageType::Disconnection => Spans::from(vec![
                    Span::styled(date, Style::default().fg(Color::DarkGray)),
                    Span::styled(&message.user, Style::default().fg(color)),
                    Span::styled(" is offline", Style::default().fg(color)),
                ]),
                MessageType::Text(content) => {
                    let mut ui_message = vec![
                        Span::styled(date, Style::default().fg(Color::DarkGray)),
                        Span::styled(&message.user, Style::default().fg(color)),
                        Span::styled(": ", Style::default().fg(color)),
                    ];
                    ui_message.extend(parse_content(content));
                    Spans::from(ui_message)
                }
                MessageType::System(content, msg_type) => {
                    let (user_color, content_color) = match msg_type {
                        SystemMessageType::Info => (Color::Cyan, Color::LightCyan),
                        SystemMessageType::Warning => (Color::Yellow, Color::LightYellow),
                        SystemMessageType::Error => (Color::Red, Color::LightRed),
                    };
                    Spans::from(vec![
                        Span::styled(date, Style::default().fg(Color::DarkGray)),
                        Span::styled(&message.user, Style::default().fg(user_color)),
                        Span::styled(content, Style::default().fg(content_color)),
                    ])
                }
                MessageType::Progress(state) => Spans::from(add_progress_bar(chunk.width, state)),
            }
        })
        .collect::<Vec<_>>();

    let messages_panel = Paragraph::new(messages)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled("LAN Room", Style::default().add_modifier(Modifier::BOLD))),
        )
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
        .scroll((state.scroll_messages_view() as u16, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(messages_panel, chunk);
}

fn add_progress_bar(panel_width: u16, progress: &ProgressState) -> Vec<Span> {
    let color = Color::LightGreen;
    let width = (panel_width - 20) as usize;

    let (title, ui_current, ui_remaining) = match progress {
        ProgressState::Started(_) => ("Pending: ", 0, width),
        ProgressState::Working(total, current) => {
            let percentage = *current as f64 / *total as f64;
            let ui_current = (percentage * width as f64) as usize;
            let ui_remaining = width - ui_current;
            ("Sending: ", ui_current, ui_remaining)
        }
        ProgressState::Completed => ("Done! ", width, 0),
    };

    let current: String = std::iter::repeat("#").take(ui_current).collect();
    let remaining: String = std::iter::repeat("-").take(ui_remaining).collect();

    let msg = format!("[{}{}]", current, remaining);
    let ui_message = vec![
        Span::styled(title, Style::default().fg(color)),
        Span::styled(msg, Style::default().fg(color)),
    ];
    ui_message
}

fn parse_content(content: &str) -> Vec<Span> {
    if content.starts_with(CommandManager::COMMAND_PREFIX) {
        // The content represents a command
        content
            .split_whitespace()
            .enumerate()
            .map(|(index, part)| {
                if index == 0 {
                    Span::styled(part, Style::default().fg(Color::LightYellow))
                }
                else {
                    Span::raw(format!(" {}", part))
                }
            })
            .collect()
    }
    else {
        vec![Span::raw(content)]
    }
}

fn draw_input_panel(frame: &mut Frame<CrosstermBackend<impl Write>>, state: &State, chunk: Rect) {
    let inner_width = (chunk.width - 2) as usize;

    let input = state.input().iter().collect::<String>();
    let input = split_each(input, inner_width)
        .into_iter()
        .map(|line| Spans::from(vec![Span::raw(line)]))
        .collect::<Vec<_>>();

    let input_panel = Paragraph::new(input)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(Span::styled("Your message", Style::default().add_modifier(Modifier::BOLD))),
        )
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);

    frame.render_widget(input_panel, chunk);

    let input_cursor = state.ui_input_cursor(inner_width);
    frame.set_cursor(chunk.x + 1 + input_cursor.0, chunk.y + 1 + input_cursor.1)
}
