use std::vec;

use enso::bundle::{actions::Action, core::ParamValue};
use ratatui::{
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, List, ListItem, ListState},
    Frame,
};

use super::KeyEvent;

pub(crate) enum Navigable<'a> {
    NotNavigable(usize),
    Navigable(KeyEvent, &'a mut usize),
}

fn handle_navigate(selected: usize, list_size: usize, event: &KeyEvent) -> usize {
    if list_size == 0 {
        return 0;
    }
    match event {
        KeyEvent::Down => (selected + 1) % list_size,
        KeyEvent::Up => (selected + list_size - 1) % list_size,
        _ => selected,
    }
}

pub(crate) fn draw_nav_list(
    f: &mut Frame,
    items: Vec<ListItem>,
    area: Rect,
    title: &str,
    navigate: Navigable,
) {
    let (selected, color) = match navigate {
        Navigable::NotNavigable(selected) => (selected, Color::White),
        Navigable::Navigable(event, selected) => {
            *selected = handle_navigate(*selected, items.len(), &event);
            (*selected, Color::Red)
        }
    };
    let list = List::new(items)
        .block(Block::default().title(title).borders(Borders::ALL))
        .style(Style::default().fg(color))
        .highlight_style(Style::default().add_modifier(Modifier::ITALIC))
        .highlight_symbol("> ");
    let mut state = ListState::default();
    state.select(Some(selected));
    f.render_stateful_widget(list, area, &mut state);
}

pub(crate) fn draw_args_list(
    f: &mut Frame,
    action: Option<&Action>,
    area: Rect,
    protocol: &str,
    navigate: Navigable,
) {
    let items = if let Some(action) = action {
        action
            .inputs
            .iter()
            .map(|(arg, _)| ListItem::new(arg.clone()))
            .collect::<Vec<ListItem>>()
    } else {
        vec![]
    };
    draw_nav_list(
        f,
        items,
        area,
        if action.is_some() {
            protocol
        } else {
            "No Action selected"
        },
        navigate,
    );
}

pub(crate) fn draw_value_list(
    f: &mut Frame,
    param: Option<&ParamValue>,
    area: Rect,
    title: &str,
    navigate: Navigable,
) {
    fn get_value(value: &ParamValue) -> Vec<ListItem> {
        match value {
            ParamValue::ValueArray(values) => values
                .iter()
                .map(|value| get_value(value)[0].clone())
                .collect::<Vec<ListItem>>(),
            ParamValue::Value(v) => vec![ListItem::new(v.clone())],
            ParamValue::Transaction(t) => vec![ListItem::new(format!("Use output at {}", t))],
            ParamValue::LastTransaction => vec![ListItem::new("Use last output")],
        }
    }
    let items = if let Some(param) = &param {
        get_value(param)
    } else {
        vec![]
    };
    draw_nav_list(
        f,
        items,
        area,
        if param.is_some() {
            title
        } else {
            "No value stored"
        },
        navigate,
    );
}

pub(crate) fn draw_tokens(
    f: &mut Frame,
    tokens: &Option<Vec<String>>,
    area: Rect,
    navigate: Navigable,
) {
    let items = if let Some(tokens) = tokens.as_ref() {
        tokens
            .iter()
            .map(|token| ListItem::new(token.as_str()))
            .collect::<Vec<ListItem>>()
    } else {
        vec![ListItem::new("Waiting tokens list...")]
    };
    draw_nav_list(f, items, area, "Tokens", navigate);
}

pub(crate) fn draw_action_type_list(f: &mut Frame, area: Rect, navigate: Navigable) {
    let items = vec![
        ListItem::new("Enso Router"),
        ListItem::new("Contract call"),
        ListItem::new("Protocol-specific"),
    ];
    draw_nav_list(f, items, area, "Type of Action", navigate);
}

pub(crate) fn draw_transactions_list(
    f: &mut Frame,
    data: &[String],
    area: Rect,
    navigate: Navigable,
) {
    let items = data
        .iter()
        .map(|action| ListItem::new(action.clone()))
        .collect::<Vec<ListItem>>();
    draw_nav_list(f, items, area, "Transactions", navigate);
}
