use std::{io, time::Duration};

use crossterm::event::{self, Event, KeyCode};
use ratatui::{
    layout::Rect,
    style::Style,
    text::Text,
    widgets::{Block, Borders, Paragraph},
    Frame,
};

pub enum InputType {
    Hex,
    Number,
    Text,
    All,
}

#[derive(Clone, Copy, Debug)]
pub enum KeyEvent {
    None,
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
    Backspace,
    Char(char),
}

pub fn poll_key_event() -> Result<KeyEvent, io::Error> {
    let key_event = match event::poll(Duration::from_millis(50)) {
        Ok(true) => {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => KeyEvent::Esc,
                    KeyCode::Char(c) => KeyEvent::Char(c),
                    KeyCode::Up => KeyEvent::Up,
                    KeyCode::Down => KeyEvent::Down,
                    KeyCode::Enter => KeyEvent::Enter,
                    KeyCode::Left => KeyEvent::Left,
                    KeyCode::Right => KeyEvent::Right,
                    KeyCode::Backspace => KeyEvent::Backspace,
                    _ => KeyEvent::None,
                }
            } else {
                KeyEvent::None
            }
        }
        _ => KeyEvent::None,
    };
    Ok(key_event)
}

pub fn draw_input(
    f: &mut Frame,
    label: &str,
    content: &mut String,
    area: Rect,
    key_event: KeyEvent,
    input_type: &InputType,
) {
    let block = Block::default().title(label).borders(Borders::ALL);
    handle_input(content, key_event, input_type);
    let text = match input_type {
        InputType::Hex => Text::from(format!("  {}: 0x{}", label, content)),
        _ => Text::from(format!("  {}: {}", label, content)),
    };
    let paragraph = Paragraph::new(text).block(block).style(Style::default());
    f.render_widget(paragraph, area);
}

fn handle_input(value: &mut String, key_event: KeyEvent, input_type: &InputType) {
    match key_event {
        KeyEvent::Char(c) => match input_type {
            InputType::Hex => {
                if c.is_ascii_hexdigit() {
                    value.push(c);
                }
            }
            InputType::Number => {
                if c.is_ascii_digit() {
                    value.push(c);
                }
            }
            InputType::Text => {
                value.push(c);
            }
            _ => (),
        },
        KeyEvent::Backspace => {
            value.pop();
        }
        _ => {}
    }
}
