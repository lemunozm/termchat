use super::util::{SplitEach};
use super::state::{ApplicationState};

use tui::{Terminal, Frame};
use tui::backend::{CrosstermBackend};
use tui::widgets::{Block, Borders, Paragraph, Wrap};
use tui::layout::{Layout, Constraint, Direction, Rect, Alignment};
use tui::style::{Style, Modifier, Color};
use tui::text::{Span, Spans};

use std::io::{Stdout};

pub fn draw(terminal: &mut Terminal<CrosstermBackend<Stdout>>, state: &ApplicationState) {
    terminal.draw(|frame| {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Min(0),
                    Constraint::Length(6)
                ].as_ref()
            )
            .split(frame.size());

        draw_messages_panel(frame, state, chunks[0]);
        draw_input_panel(frame, state, chunks[1]);

    }).unwrap()
}

fn draw_messages_panel(frame: &mut Frame<CrosstermBackend<Stdout>>, state: &ApplicationState, chunk: Rect) {
    let messages = state
        .messages()
        .iter()
        .rev()
        .map(|message| {
            let date = message.date.format("%H:%M:%S ").to_string();
            Spans::from(vec![
                Span::styled(date, Style::default().fg(Color::DarkGray)),
                Span::styled(&message.user, Style::default().fg(Color::Green)),
                Span::styled(": ", Style::default().fg(Color::Green)),
                Span::raw(&message.msg),
            ])
        })
        .collect::<Vec<_>>();

    let messages_panel = Paragraph::new(messages)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                "LAN Room",
                Style::default().add_modifier(Modifier::BOLD)
            )))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left)
        .scroll((state.scroll_messages_view() as u16, 0))
        .wrap(Wrap { trim: false });

    frame.render_widget(messages_panel, chunk);
}

fn draw_input_panel(frame: &mut Frame<CrosstermBackend<Stdout>>, state: &ApplicationState, chunk: Rect) {
    let inner_width = (chunk.width - 2) as usize;

    let input = state
        .input()
        .split_each(inner_width)
        .iter()
        .map(|line| {
            Spans::from(vec![
                Span::raw(*line),
            ])
        })
        .collect::<Vec<_>>();

    let input_panel = Paragraph::new(input)
        .block(Block::default()
            .borders(Borders::ALL)
            .title(Span::styled(
                "Your message",
                Style::default().add_modifier(Modifier::BOLD)
            )))
        .style(Style::default().fg(Color::White))
        .alignment(Alignment::Left);

    frame.render_widget(input_panel, chunk);

    frame.set_cursor(
        chunk.x + 1 + (state.input_cursor() % inner_width) as u16,
        chunk.y + 1 + (state.input_cursor() / inner_width) as u16,
    )
}
