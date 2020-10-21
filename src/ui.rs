use super::state::{ApplicationState, MessageType};
use super::util::SplitEach;
use crate::util::Result;

use tui::backend::CrosstermBackend;
use tui::layout::{Alignment, Constraint, Direction, Layout, Rect};
use tui::style::{Color, Modifier, Style};
use tui::text::{Span, Spans};
use tui::widgets::{Block, Borders, Paragraph, Wrap};
use tui::{Frame, Terminal};

use std::io::Stdout;

pub fn draw(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    state: &ApplicationState,
) -> Result<()> {
    Ok(terminal.draw(|frame| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(0), Constraint::Length(6)].as_ref())
            .split(frame.size());

        draw_messages_panel(frame, state, chunks[0]);
        draw_input_panel(frame, state, chunks[1]);
    })?)
}

fn draw_messages_panel(
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    state: &ApplicationState,
    chunk: Rect,
) {
    const MESSAGE_COLORS: [Color; 4] = [Color::Blue, Color::Yellow, Color::Cyan, Color::Magenta];

    let messages = state
        .messages()
        .iter()
        .rev()
        .map(|message| {
            let color = if let Some(id) = state.users_id().get(&message.user) {
                MESSAGE_COLORS[id % MESSAGE_COLORS.len()]
            } else {
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
                MessageType::Content(content) => Spans::from(vec![
                    Span::styled(date, Style::default().fg(Color::DarkGray)),
                    Span::styled(&message.user, Style::default().fg(color)),
                    Span::styled(": ", Style::default().fg(color)),
                    Span::raw(content),
                ]),
                MessageType::Error(error) => Spans::from(vec![
                    Span::styled(date, Style::default().fg(Color::DarkGray)),
                    Span::styled(&message.user, Style::default().fg(Color::Red)),
                    Span::styled(error, Style::default().fg(Color::LightRed)),
                ]),
            }
        })
        .collect::<Vec<_>>();

    let messages_panel = Paragraph::new(messages)
        .block(Block::default().borders(Borders::ALL).title(Span::styled(
            "LAN Room",
            Style::default().add_modifier(Modifier::BOLD),
        )))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
        .scroll((state.scroll_messages_view() as u16, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(messages_panel, chunk);
}

fn draw_input_panel(
    frame: &mut Frame<CrosstermBackend<Stdout>>,
    state: &ApplicationState,
    chunk: Rect,
) {
    let inner_width = (chunk.width - 2) as usize;

    let input = state.input().iter().collect::<String>();
    let input = input
        .split_each(inner_width)
        .into_iter()
        .map(|line| Spans::from(vec![Span::raw(line)]))
        .collect::<Vec<_>>();

    let input_panel = Paragraph::new(input)
        .block(Block::default().borders(Borders::ALL).title(Span::styled(
            "Your message",
            Style::default().add_modifier(Modifier::BOLD),
        )))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);

    frame.render_widget(input_panel, chunk);

    let input_cursor = state.ui_input_cursor(inner_width);
    frame.set_cursor(chunk.x + 1 + input_cursor.0, chunk.y + 1 + input_cursor.1)
}
