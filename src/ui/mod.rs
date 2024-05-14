use std::{io, rc::Rc, time::Duration};

use crossterm::{
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use enso::{
    bundle::{
        actions::{Action, ACTION_CALL},
        core::ParamValue,
    },
    metadata::{
        networks::Network,
        protocols::{Protocol, ENSO_PROTOCOL},
    },
};
use once_cell::sync::Lazy;
use ratatui::{
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Style, Stylize},
    text::Line,
    widgets::{Block, Borders, ListItem, Paragraph},
    Frame, Terminal,
};
use tokio::{
    sync::mpsc::{Receiver, Sender},
    time,
};

use crate::{ui::keyboard::InputType, BusinessResponse, UIRequest};

use self::{
    basic_drawings::{
        draw_action_type_list, draw_args_list, draw_nav_list, draw_tokens, draw_transactions_list,
        draw_value_list, Navigable,
    },
    keyboard::{draw_input, poll_key_event, KeyEvent},
};

mod basic_drawings;
mod keyboard;

enum UIState {
    NetworkSelector {
        selected_network: usize,
    },
    BrowseTransactions,
    BrowseParameters,
    BrowseValues,
    ActionTypeSelector(usize),
    ProtocolSelector {
        selected_protocol: usize,
        selected_action_type: usize,
    },
    ActionSelector {
        protocol: Protocol,
        selected_action_type: usize,
        selected_action: usize,
    },
    TokenSelector {
        selected_token: usize,
    },
    ArgumentInput {
        selecting_type: bool,
        input_type: InputType,
        amount_type_selected: usize,
        content: String,
    },
}

struct Handle<'a, 'b> {
    f: &'b mut Frame<'a>,
    data: &'b mut Data,
    header: Rect,
    body: Rc<[Rect]>,
    footer: Rect,
    key_event: KeyEvent,
}

pub type DataTransaction = Vec<(Action, Protocol, Vec<ParamValue>)>;

static H_NETWORK_DESC: Lazy<Paragraph> = Lazy::new(|| {
    let block = Block::default()
        .title("Enso, create and send bundle transactions.")
        .borders(Borders::ALL);
    let text: Vec<Line> = vec!["".into(), "Select a network".into()];
    Paragraph::new(text).block(block).style(Style::default())
});

static H_HOME_DESC: Lazy<Paragraph> = Lazy::new(|| {
    let block = Block::default()
        .title("Enso, create and send bundle transactions.")
        .borders(Borders::ALL);
    let text: Vec<Line> = vec![Line::from(""), Line::from("Enso").bold()];
    Paragraph::new(text).block(block).style(Style::default())
});

static H_TX_DESC: Lazy<Paragraph> = Lazy::new(|| {
    let block = Block::default()
        .title("Enso, create and send bundle transactions.")
        .borders(Borders::ALL);
    let text: Vec<Line> = vec![
        vec!["Enter | →".bold(), ": Edit the current item".into()].into(),
        vec!["ESC".bold(), ": Exit application".into()].into(),
        vec!["S".bold(), ": Send bundle and start a new one".into()].into(),
        vec!["I".bold(), ": Insert a new transaction".into()].into(),
        vec!["D".bold(), ": Delete current transaction".into()].into(),
    ];
    Paragraph::new(text).block(block).style(Style::default())
});

static H_PARAMS_DESC: Lazy<Paragraph> = Lazy::new(|| {
    let block = Block::default()
        .title("Enso, create and send bundle transactions.")
        .borders(Borders::ALL);
    let text: Vec<Line> = vec![
        vec!["Enter | →".bold(), ": Edit the current item".into()].into(),
        vec!["ESC | ←".bold(), ": Back to transactions list".into()].into(),
        vec!["S".bold(), ": Send bundle and start a new one".into()].into(),
        vec!["I".bold(), ": Insert a new transaction".into()].into(),
    ];
    Paragraph::new(text).block(block).style(Style::default())
});

static H_VALUE_DESC: Lazy<Paragraph> = Lazy::new(|| {
    let block = Block::default()
        .title("Enso, create and send bundle transactions.")
        .borders(Borders::ALL);
    let text: Vec<Line> = vec![
        vec!["Enter".bold(), ": Edit the current item".into()].into(),
        vec!["ESC | ←".bold(), ": Back to parameters list".into()].into(),
        vec!["S".bold(), ": Send bundle and start a new one".into()].into(),
        vec![
            "I".bold(),
            ": Insert a new arg for an `args` parameter".into(),
        ]
        .into(),
    ];
    Paragraph::new(text).block(block).style(Style::default())
});

static H_ACTION_TYPE_DESC: Lazy<Paragraph> = Lazy::new(|| {
    let block = Block::default()
        .title("Enso, create and send bundle transactions.")
        .borders(Borders::ALL);
    let text: Vec<Line> = vec![
        "".into(),
        "Select what type of action you want to add".into(),
    ];
    Paragraph::new(text).block(block).style(Style::default())
});

static H_PROTOCOL_DESC: Lazy<Paragraph> = Lazy::new(|| {
    let block = Block::default()
        .title("Enso, create and send bundle transactions.")
        .borders(Borders::ALL);
    let text: Vec<Line> = vec![
        "".into(),
        "Select the protocol on which you would like to perform an action".into(),
    ];
    Paragraph::new(text).block(block).style(Style::default())
});

static H_ACTION_DESC: Lazy<Paragraph> = Lazy::new(|| {
    let block = Block::default()
        .title("Enso, create and send bundle transactions.")
        .borders(Borders::ALL);
    let text: Vec<Line> = vec![
        "".into(),
        "Select the action you would like to perform on the chosen protocol".into(),
    ];
    Paragraph::new(text).block(block).style(Style::default())
});

static H_TOKEN_DESC: Lazy<Paragraph> = Lazy::new(|| {
    let block = Block::default()
        .title("Enso, create and send bundle transactions.")
        .borders(Borders::ALL);
    let text: Vec<Line> = vec!["".into(), "Select a token".into()];
    Paragraph::new(text).block(block).style(Style::default())
});

#[derive(Default)]
struct Data {
    transactions: DataTransaction,
    selected_transaction: usize,
    selected_parameter: usize,
    selected_value: usize,
}

#[derive(Default)]
struct Cache {
    tokens: Option<Vec<String>>,
    protocols: Option<Vec<Protocol>>,
    actions: Option<Vec<Action>>,
    networks: Option<Vec<Network>>,
}

pub async fn run(
    ui_to_business_sender: Sender<UIRequest>,
    mut business_to_ui_receiver: Receiver<BusinessResponse>,
) -> Result<(), io::Error> {
    enable_raw_mode()?;
    let mut stdout = io::stdout();
    execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;

    let mut update_ui = true;
    let mut ui_state = UIState::NetworkSelector {
        selected_network: 0,
    };
    let mut key_event = KeyEvent::None;
    let mut data = Data::default();
    let mut cache = Cache::default();

    _ = ui_to_business_sender.send(UIRequest::GetNetworks).await;

    loop {
        let mut msg = None;
        if update_ui {
            terminal.draw(|f| {
                msg = layout(f, &mut ui_state, &mut data, key_event, &cache);
            })?;
        }
        if let Some(msg) = msg {
            _ = ui_to_business_sender.send(msg).await;
        }

        key_event = poll_key_event()?;
        match (key_event, &ui_state) {
            (KeyEvent::Esc, UIState::BrowseTransactions) => break,
            (KeyEvent::None, _) => update_ui = true,
            _ => update_ui = true,
        }
        match time::timeout(Duration::from_millis(10), business_to_ui_receiver.recv()).await {
            Ok(Some(BusinessResponse::Protocols(p))) => {
                cache.protocols = Some(p);
            }
            Ok(Some(BusinessResponse::Actions(a))) => {
                cache.actions = Some(a);
            }
            Ok(Some(BusinessResponse::Tokens(t))) => {
                cache.tokens = Some(t);
            }
            Ok(Some(BusinessResponse::Networks(t))) => {
                cache.networks = Some(t);
            }
            _ => {}
        }
    }
    _ = ui_to_business_sender.send(UIRequest::Quit).await;
    disable_raw_mode()?;
    execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;
    Ok(())
}

fn layout(
    f: &mut Frame,
    mut ui_state: &mut UIState,
    data: &mut Data,
    key_event: KeyEvent,
    cache: &Cache,
) -> Option<UIRequest> {
    let Cache {
        protocols,
        tokens,
        actions,
        networks,
    } = cache;
    let header = Layout::default()
        .direction(Direction::Vertical)
        .margin(1)
        .constraints(
            [
                Constraint::Percentage(20),
                Constraint::Percentage(70),
                Constraint::Percentage(10),
            ]
            .as_ref(),
        )
        .split(f.size());
    let body = Layout::default()
        .direction(Direction::Horizontal)
        .margin(0)
        .constraints(
            [
                Constraint::Percentage(20),
                Constraint::Percentage(41),
                Constraint::Percentage(40),
            ]
            .as_ref(),
        )
        .split(header[1]);
    let footer = Layout::default()
        .direction(Direction::Horizontal)
        .margin(0)
        .constraints([Constraint::Percentage(100)].as_ref())
        .split(header[2]);

    f.render_widget(H_HOME_DESC.clone(), header[0]);

    match &mut ui_state {
        UIState::NetworkSelector { selected_network } => {
            let request = handle_network_selector(
                Handle {
                    f,
                    data,
                    header: header[0],
                    body,
                    footer: footer[0],
                    key_event,
                },
                networks,
                selected_network,
            );
            return request.map(|r| {
                *ui_state = UIState::BrowseTransactions;
                r
            });
        }
        UIState::BrowseTransactions | UIState::BrowseParameters | UIState::BrowseValues => {
            let request = browse_transactions(
                Handle {
                    f,
                    data,
                    header: header[0],
                    body,
                    footer: footer[0],
                    key_event,
                },
                ui_state,
            );
            if let (Some(request), None) = (request, tokens) {
                return Some(request);
            }
        }
        UIState::ActionTypeSelector(selected) => {
            let state = handle_action_type_selection(
                Handle {
                    f,
                    data,
                    header: header[0],
                    body,
                    footer: footer[0],
                    key_event,
                },
                selected,
                key_event,
            );
            if let Some(state) = state {
                let request = match state {
                    UIState::ProtocolSelector { .. } if protocols.is_none() => {
                        Some(UIRequest::GetProtocols)
                    }
                    _ => None,
                };
                *ui_state = state;
                return request;
            }
        }
        UIState::ProtocolSelector {
            selected_action_type,
            selected_protocol,
        } => {
            let state = handle_protocol_selection(
                Handle {
                    f,
                    data,
                    header: header[0],
                    body,
                    footer: footer[0],
                    key_event,
                },
                protocols,
                *selected_action_type,
                selected_protocol,
            );
            if let Some(state) = state {
                let request = match state {
                    UIState::ActionSelector { .. } if actions.is_none() => {
                        Some(UIRequest::GetActions)
                    }
                    _ => None,
                };
                *ui_state = state;
                return request;
            }
        }
        UIState::ActionSelector {
            protocol,
            selected_action_type,
            selected_action,
        } => {
            let state = handle_action_selection(
                Handle {
                    f,
                    data,
                    header: header[0],
                    body,
                    footer: footer[0],
                    key_event,
                },
                actions,
                *selected_action_type,
                selected_action,
                protocol,
            );
            if let Some(state) = state {
                *ui_state = state;
            }
        }
        UIState::TokenSelector { selected_token } => {
            let state = handle_token_selection(
                Handle {
                    f,
                    data,
                    header: header[0],
                    body,
                    footer: footer[0],
                    key_event,
                },
                tokens,
                selected_token,
            );
            if let Some(state) = state {
                *ui_state = state;
            }
        }
        UIState::ArgumentInput {
            input_type,
            amount_type_selected,
            content,
            selecting_type,
        } => {
            if let Some(state) = handle_args_input(
                Handle {
                    f,
                    data,
                    header: header[0],
                    body,
                    footer: footer[0],
                    key_event,
                },
                content,
                input_type,
                amount_type_selected,
                selecting_type,
            ) {
                *ui_state = state;
            }
        }
    };
    None
}

fn handle_network_selector(
    h: Handle,
    networks: &Option<Vec<Network>>,
    selected_network: &mut usize,
) -> Option<UIRequest> {
    h.f.render_widget(H_NETWORK_DESC.clone(), h.header);
    let items = if let Some(networks) = networks {
        networks
            .iter()
            .map(|network| ListItem::new(network.name.clone()))
            .collect::<Vec<ListItem>>()
    } else {
        vec![ListItem::new("Waiting protocols list...")]
    };
    draw_nav_list(
        h.f,
        items,
        h.body[0],
        "Networks",
        Navigable::Navigable(h.key_event, selected_network),
    );
    h.f.render_widget(Block::default().borders(Borders::ALL), h.body[1]);
    h.f.render_widget(Block::default().borders(Borders::ALL), h.body[2]);
    match h.key_event {
        KeyEvent::Enter | KeyEvent::Right => networks
            .as_ref()
            .and_then(|n| n.get(*selected_network))
            .map(|network| UIRequest::SetNetwork(network.id)),
        _ => None,
    }
}

fn browse_transactions(h: Handle, ui_state: &mut UIState) -> Option<UIRequest> {
    match ui_state {
        UIState::BrowseTransactions => h.f.render_widget(H_TX_DESC.clone(), h.header),
        UIState::BrowseParameters => h.f.render_widget(H_PARAMS_DESC.clone(), h.header),
        UIState::BrowseValues => h.f.render_widget(H_VALUE_DESC.clone(), h.header),
        _ => (),
    }
    let transactions = h
        .data
        .transactions
        .iter()
        .map(|(tx, _, _)| tx.action.clone())
        .collect::<Vec<_>>();
    let last_selected = h.data.selected_transaction;
    let navigate = if let UIState::BrowseTransactions = ui_state {
        Navigable::Navigable(h.key_event, &mut h.data.selected_transaction)
    } else {
        Navigable::NotNavigable(h.data.selected_transaction)
    };
    draw_transactions_list(h.f, &transactions, h.body[0], navigate);
    if last_selected != h.data.selected_transaction {
        h.data.selected_parameter = 0;
        h.data.selected_value = 0;
    }

    let result = h
        .data
        .transactions
        .get_mut(h.data.selected_transaction)
        .map(|tx| (&tx.0, &tx.1, &mut tx.2));
    let (action, protocol, mut param) = match result {
        Some((action, protocol, param)) => (
            Some(action),
            Some(protocol),
            param.get_mut(h.data.selected_parameter),
        ),
        None => (None, None, None),
    };
    let protocol = protocol.map(|p| p.slug.as_str()).unwrap_or("No protocol");
    let last_selected = h.data.selected_parameter;
    let navigate = if let UIState::BrowseParameters = ui_state {
        Navigable::Navigable(h.key_event, &mut h.data.selected_parameter)
    } else {
        Navigable::NotNavigable(h.data.selected_parameter)
    };
    draw_args_list(h.f, action, h.body[1], protocol, navigate);
    if last_selected != h.data.selected_parameter {
        h.data.selected_value = 0;
    }

    let title = action
        .map(|a| a.inputs[h.data.selected_parameter].1.clone())
        .unwrap_or("No parameter selected".to_string());
    let navigate = if let UIState::BrowseValues = ui_state {
        Navigable::Navigable(h.key_event, &mut h.data.selected_value)
    } else {
        Navigable::NotNavigable(h.data.selected_value)
    };
    draw_value_list(h.f, param.as_deref(), h.body[2], &title, navigate);

    match (h.key_event, &ui_state) {
        (KeyEvent::Enter | KeyEvent::Right, UIState::BrowseTransactions) => {
            *ui_state = UIState::BrowseParameters
        }
        (KeyEvent::Enter | KeyEvent::Right, UIState::BrowseParameters) => {
            *ui_state = UIState::BrowseValues
        }
        (KeyEvent::Enter, UIState::BrowseValues) => {
            enum ArgType {
                Token,
                Address,
                Value,
                Args,
                Text,
            }
            match action
                .and_then(|a| a.inputs.get(h.data.selected_parameter))
                .map(|(f, _)| {
                    if f.to_lowercase().contains("token") {
                        ArgType::Token
                    } else if f.to_lowercase().contains("address") {
                        ArgType::Address
                    } else if f.to_lowercase() == "method" || f.to_lowercase() == "abi" {
                        ArgType::Text
                    } else if f.to_lowercase() == "args" {
                        ArgType::Args
                    } else {
                        ArgType::Value
                    }
                }) {
                Some(ArgType::Token) => {
                    *ui_state = UIState::TokenSelector { selected_token: 0 };
                    return Some(UIRequest::GetTokens);
                }
                Some(ArgType::Address) => {
                    *ui_state = UIState::ArgumentInput {
                        selecting_type: false,
                        input_type: InputType::Hex,
                        amount_type_selected: 0,
                        content: String::new(),
                    };
                    return None;
                }
                Some(ArgType::Value) => {
                    *ui_state = UIState::ArgumentInput {
                        selecting_type: true,
                        input_type: InputType::Number,
                        amount_type_selected: 0,
                        content: String::new(),
                    };
                    return None;
                }
                Some(ArgType::Text) => {
                    *ui_state = UIState::ArgumentInput {
                        selecting_type: false,
                        input_type: InputType::Text,
                        amount_type_selected: 0,
                        content: String::new(),
                    };
                    return None;
                }
                Some(ArgType::Args) => {
                    if let Some(ParamValue::ValueArray(params)) = param {
                        if !params.is_empty() {
                            *ui_state = UIState::ArgumentInput {
                                selecting_type: true,
                                input_type: InputType::All,
                                amount_type_selected: 0,
                                content: String::new(),
                            };
                        }
                    }
                    return None;
                }
                _ => return None,
            }
        }
        (KeyEvent::Esc | KeyEvent::Left, UIState::BrowseParameters) => {
            *ui_state = UIState::BrowseTransactions;
        }
        (KeyEvent::Esc | KeyEvent::Left, UIState::BrowseValues) => {
            *ui_state = UIState::BrowseParameters;
        }
        (KeyEvent::Char('I') | KeyEvent::Char('i'), UIState::BrowseValues) => {
            if let Some(ParamValue::ValueArray(params)) = param.as_mut() {
                params.push(ParamValue::Value("''".to_owned()));
            }
        }
        (KeyEvent::Char('I') | KeyEvent::Char('i'), _) => {
            *ui_state = UIState::ActionTypeSelector(0);
        }
        (KeyEvent::Char('E') | KeyEvent::Char('e'), _) => {
            if !h.data.transactions.is_empty() {
                let transactions = h.data.transactions.clone();
                h.data.transactions.clear();
                h.data.selected_transaction = 0;
                h.data.selected_parameter = 0;
                h.data.selected_value = 0;
                *ui_state = UIState::BrowseTransactions;
                return Some(UIRequest::SendBundle(transactions));
            }
        }
        (KeyEvent::Char('D') | KeyEvent::Char('d'), UIState::BrowseTransactions) => {
            if !h.data.transactions.is_empty() {
                h.data.transactions.remove(h.data.selected_transaction);
                if h.data.selected_transaction >= h.data.transactions.len() {
                    h.data.selected_transaction = h.data.transactions.len().saturating_sub(1);
                }
                h.data.selected_parameter = 0;
                h.data.selected_value = 0;
            }
        }
        _ => {}
    };
    None
}

fn handle_action_type_selection(
    h: Handle,
    selected: &mut usize,
    key_event: KeyEvent,
) -> Option<UIState> {
    h.f.render_widget(H_ACTION_TYPE_DESC.clone(), h.header);
    draw_action_type_list(h.f, h.body[0], Navigable::Navigable(key_event, selected));
    h.f.render_widget(Block::default().borders(Borders::ALL), h.body[1]);
    h.f.render_widget(Block::default().borders(Borders::ALL), h.body[2]);

    match key_event {
        KeyEvent::Enter | KeyEvent::Right => match *selected {
            0 => Some(UIState::ProtocolSelector {
                selected_protocol: 0,
                selected_action_type: 0,
            }),
            1 => {
                h.data.transactions.push((
                    ACTION_CALL.clone(),
                    ENSO_PROTOCOL.clone(),
                    set_default_param_values(&ACTION_CALL),
                ));

                h.data.selected_transaction = h.data.transactions.len() - 1;
                h.data.selected_parameter = 0;
                h.data.selected_value = 0;
                Some(UIState::BrowseParameters)
            }
            2 => None,
            _ => None,
        },
        _ => None,
    }
}

fn handle_protocol_selection(
    h: Handle,
    protocols: &Option<Vec<Protocol>>,
    selected_action_type: usize,
    selected_protocol: &mut usize,
) -> Option<UIState> {
    h.f.render_widget(H_PROTOCOL_DESC.clone(), h.header);
    draw_action_type_list(
        h.f,
        h.body[0],
        Navigable::NotNavigable(selected_action_type),
    );
    let items = if let Some(protocols) = protocols {
        protocols
            .iter()
            .map(|protocol| ListItem::new(protocol.slug.clone()))
            .collect::<Vec<ListItem>>()
    } else {
        vec![ListItem::new("Waiting protocols list...")]
    };
    draw_nav_list(
        h.f,
        items,
        h.body[1],
        "Protocols",
        Navigable::Navigable(h.key_event, selected_protocol),
    );
    h.f.render_widget(Block::default().borders(Borders::ALL), h.body[2]);
    match h.key_event {
        KeyEvent::Enter | KeyEvent::Right => protocols
            .as_ref()
            .and_then(|p| p.get(*selected_protocol))
            .map(|protocol| UIState::ActionSelector {
                protocol: protocol.clone(),
                selected_action_type,
                selected_action: 0,
            }),
        _ => None,
    }
}

fn handle_action_selection(
    h: Handle,
    actions: &Option<Vec<Action>>,
    selected_action_type: usize,
    selected_action: &mut usize,
    protocol: &Protocol,
) -> Option<UIState> {
    h.f.render_widget(H_ACTION_DESC.clone(), h.header);
    draw_action_type_list(
        h.f,
        h.body[0],
        Navigable::NotNavigable(selected_action_type),
    );
    let items = if let Some(actions) = actions {
        actions
            .iter()
            .map(|action| ListItem::new(action.action.clone()))
            .collect::<Vec<ListItem>>()
    } else {
        vec![ListItem::new("Waiting actions list...")]
    };
    draw_nav_list(
        h.f,
        items,
        h.body[1],
        "Actions",
        Navigable::Navigable(h.key_event, selected_action),
    );
    h.f.render_widget(Block::default().borders(Borders::ALL), h.body[2]);
    match h.key_event {
        KeyEvent::Enter | KeyEvent::Right => {
            if let Some(action) = actions.as_ref().and_then(|p| p.get(*selected_action)) {
                h.data.transactions.push((
                    action.clone(),
                    protocol.clone(),
                    set_default_param_values(action),
                ));
                h.data.selected_transaction = h.data.transactions.len() - 1;
                h.data.selected_parameter = 0;
                h.data.selected_value = 0;
                Some(UIState::BrowseParameters)
            } else {
                None
            }
        }
        _ => None,
    }
}

fn handle_token_selection(
    h: Handle,
    tokens: &Option<Vec<String>>,
    selected_token: &mut usize,
) -> Option<UIState> {
    h.f.render_widget(H_TOKEN_DESC.clone(), h.header);
    let transactions = h
        .data
        .transactions
        .iter()
        .map(|(tx, _, _)| tx.action.clone())
        .collect::<Vec<_>>();
    draw_transactions_list(
        h.f,
        &transactions,
        h.body[0],
        Navigable::NotNavigable(h.data.selected_transaction),
    );
    let result = h
        .data
        .transactions
        .get(h.data.selected_transaction)
        .map(|tx| (&tx.0, &tx.1));
    let (action, protocol) = match result {
        Some((action, protocol)) => (Some(action), Some(protocol)),
        None => (None, None),
    };
    let protocol = protocol.map(|p| p.slug.as_str()).unwrap_or("No protocol");
    draw_args_list(
        h.f,
        action,
        h.body[1],
        protocol,
        Navigable::NotNavigable(h.data.selected_parameter),
    );
    let navigate = Navigable::Navigable(h.key_event, selected_token);
    draw_tokens(h.f, tokens, h.body[2], navigate);
    match h.key_event {
        KeyEvent::Enter => {
            if let Some(token) = tokens.as_ref().and_then(|t| t.get(*selected_token)) {
                let param = h
                    .data
                    .transactions
                    .get_mut(h.data.selected_transaction)
                    .and_then(|tx| tx.2.get_mut(h.data.selected_parameter));
                if let Some(param) = param {
                    *param = ParamValue::Value(token.clone());
                }
                Some(UIState::BrowseParameters)
            } else {
                None
            }
        }
        KeyEvent::Esc | KeyEvent::Left => Some(UIState::BrowseParameters),
        _ => None,
    }
}

fn set_default_param_values(action: &Action) -> Vec<ParamValue> {
    action
        .inputs
        .iter()
        .map(|(param, _)| {
            let param = param.to_lowercase();
            if param.contains("token") || param.contains("address") {
                ParamValue::Value("0x".to_owned())
            } else if param == "args" {
                ParamValue::ValueArray(Vec::new())
            } else {
                ParamValue::Value("0".to_owned())
            }
        })
        .collect::<Vec<ParamValue>>()
}

fn handle_args_input(
    handle: Handle,
    content: &mut String,
    input_type: &mut InputType,
    amount_type_selected: &mut usize,
    is_selecting_type: &mut bool,
) -> Option<UIState> {
    let transactions = handle
        .data
        .transactions
        .iter()
        .map(|(tx, _, _)| tx.action.clone())
        .collect::<Vec<_>>();
    draw_transactions_list(
        handle.f,
        &transactions,
        handle.body[0],
        Navigable::NotNavigable(handle.data.selected_transaction),
    );
    let result = handle
        .data
        .transactions
        .get_mut(handle.data.selected_transaction)
        .map(|(action, protocol, params)| (action, protocol, params));
    let (action, protocol, param) = match result {
        Some((action, protocol, param)) => (
            Some(action),
            Some(protocol),
            param.get_mut(handle.data.selected_parameter),
        ),
        None => (None, None, None),
    };
    draw_args_list(
        handle.f,
        action.as_deref(),
        handle.body[1],
        protocol
            .map(|p| p.slug.as_str())
            .unwrap_or("No transaction selected"),
        Navigable::NotNavigable(handle.data.selected_parameter),
    );
    let (input_title, list_title) = action
        .and_then(|a| a.inputs.get(handle.data.selected_parameter))
        .map(|(param, desc)| (param.to_owned(), desc.to_owned()))
        .map(|(param, desc)| (Some(param), desc))
        .unwrap_or((None, "No transaction selected".to_owned()));
    if *is_selecting_type {
        let items = if let InputType::All = input_type {
            vec![
                ListItem::new("Output of the last transaction"),
                ListItem::new("Output of transaction at"),
                ListItem::new("Value"),
                ListItem::new("Text"),
                ListItem::new("Address"),
            ]
        } else {
            vec![
                ListItem::new("Output of the last transaction"),
                ListItem::new("Output of transaction at"),
                ListItem::new("Value"),
            ]
        };
        draw_nav_list(
            handle.f,
            items,
            handle.body[2],
            "Select Input mode",
            Navigable::Navigable(handle.key_event, amount_type_selected),
        );
        match (handle.key_event, amount_type_selected) {
            (KeyEvent::Enter, 0) => {
                match param {
                    Some(ParamValue::ValueArray(param)) => {
                        param[handle.data.selected_value] = ParamValue::LastTransaction;
                    }
                    Some(param) => *param = ParamValue::LastTransaction,
                    None => (),
                }
                Some(UIState::BrowseValues)
            }
            (KeyEvent::Enter, 1) => {
                match param {
                    Some(ParamValue::ValueArray(param)) => {
                        param[handle.data.selected_value] = ParamValue::Transaction(0);
                    }
                    Some(param) => *param = ParamValue::Transaction(0),
                    None => (),
                }
                *is_selecting_type = false;
                *input_type = InputType::Number;
                None
            }
            (KeyEvent::Enter, index @ 2..=4) => {
                match param {
                    Some(ParamValue::ValueArray(param)) => {
                        param[handle.data.selected_value] = ParamValue::Value("0".to_owned());
                    }
                    Some(param) => *param = ParamValue::Value("0".to_owned()),
                    None => (),
                }
                match *index {
                    2 => *input_type = InputType::Number,
                    3 => *input_type = InputType::Text,
                    4 => *input_type = InputType::Hex,
                    _ => (),
                }
                *is_selecting_type = false;
                None
            }
            (KeyEvent::Esc, _) => Some(UIState::BrowseValues),
            _ => None,
        }
    } else {
        draw_value_list(
            handle.f,
            param.as_deref(),
            handle.body[2],
            &list_title,
            Navigable::NotNavigable(handle.data.selected_value),
        );
        if let Some(title) = &input_title {
            draw_input(
                handle.f,
                title,
                content,
                handle.footer,
                handle.key_event,
                input_type,
            );
            match handle.key_event {
                KeyEvent::Enter => {
                    if let Some(param) = param {
                        match param {
                            ParamValue::Transaction(_) => {
                                *param =
                                    ParamValue::Transaction(content.parse::<usize>().unwrap_or(0))
                            }
                            ParamValue::Value(_) => {
                                if let InputType::Hex = input_type {
                                    if !content.starts_with("0x") {
                                        content.insert_str(0, "0x");
                                    }
                                }

                                *param = ParamValue::Value(content.to_string())
                            }
                            ParamValue::ValueArray(args) => {
                                match args[handle.data.selected_value] {
                                    ParamValue::Transaction(_) => {
                                        args[handle.data.selected_value] = ParamValue::Transaction(
                                            content.parse::<usize>().unwrap_or(0),
                                        )
                                    }
                                    ParamValue::Value(_) => {
                                        if let InputType::Hex = input_type {
                                            if !content.starts_with("0x") {
                                                content.insert_str(0, "0x");
                                            }
                                        }
                                        args[handle.data.selected_value] =
                                            ParamValue::Value(content.to_string())
                                    }
                                    _ => (),
                                }
                            }
                            _ => (),
                        }
                    }
                    Some(UIState::BrowseValues)
                }
                KeyEvent::Esc => Some(UIState::BrowseValues),
                _ => None,
            }
        } else {
            None
        }
    }
}
