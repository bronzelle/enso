use anyhow::Result;

use enso::{
    bundle::{actions::Action, core::Bundle},
    core::{Enso, Version},
    metadata::{networks::Network, protocols::Protocol},
};
use futures::StreamExt;
use tokio::{
    spawn,
    sync::mpsc::{self, Receiver, Sender},
};
use ui::DataTransaction;

mod config;
mod ui;

#[tokio::main]
async fn main() -> Result<()> {
    let (ui_to_business_sender, ui_to_business_receiver) = mpsc::channel::<UIRequest>(32);
    let (business_to_ui_sender, business_to_ui_receiver) = mpsc::channel::<BusinessResponse>(32);

    let business_thread = spawn(async {
        business(business_to_ui_sender, ui_to_business_receiver).await;
    });
    let ui_thread = spawn(async {
        _ = ui::run(ui_to_business_sender, business_to_ui_receiver).await;
    });

    _ = ui_thread.await;
    _ = business_thread.await;

    Ok(())
}

#[derive(Debug)]
pub enum UIRequest {
    GetNetworks,
    SetNetwork(u32),
    GetTokens,
    GetProtocols,
    GetActions,
    SendBundle(DataTransaction),
    Quit,
}

pub enum BusinessResponse {
    Tokens(Vec<String>),
    Protocols(Vec<Protocol>),
    Actions(Vec<Action>),
    Networks(Vec<Network>),
}

async fn business(
    business_to_ui_sender: Sender<BusinessResponse>,
    mut ui_to_business_receiver: Receiver<UIRequest>,
) {
    let config = config::Config::default();
    let enso = Enso::new(config.api_key, Version::V1);
    let mut chain_id: Option<u32> = None;

    loop {
        match ui_to_business_receiver.recv().await {
            Some(UIRequest::GetTokens) => {
                let mut tokens = Vec::new();
                let mut tokens_streams =
                    enso.tokens_stream(&[("chainId", &format!("{}", chain_id.unwrap_or(1)))]);
                while let Some(tokens_received) = tokens_streams.next().await {
                    match tokens_received {
                        Ok(tokens_received) => tokens.extend(tokens_received),
                        Err(e) => println!("{:?}", e),
                    }
                }

                business_to_ui_sender
                    .send(BusinessResponse::Tokens(tokens))
                    .await
                    .unwrap();
            }
            Some(UIRequest::GetProtocols) => {
                let protocols = enso.get_protocols().await.unwrap();
                business_to_ui_sender
                    .send(BusinessResponse::Protocols(protocols))
                    .await
                    .unwrap();
            }
            Some(UIRequest::GetActions) => {
                let actions = enso.get_actions().await.unwrap();
                business_to_ui_sender
                    .send(BusinessResponse::Actions(actions))
                    .await
                    .unwrap();
            }
            Some(UIRequest::SendBundle(data)) => {
                let mut bundle = Bundle::new(1);
                data.into_iter().for_each(|(action, protocol, args)| {
                    bundle.add_action(protocol, action, args);
                });
                let _ = enso.send_bundle(bundle, "0x").await;
            }
            Some(UIRequest::GetNetworks) => {
                let networks = enso.get_networks().await.unwrap();
                business_to_ui_sender
                    .send(BusinessResponse::Networks(networks))
                    .await
                    .unwrap();
            }
            Some(UIRequest::SetNetwork(id)) => {
                chain_id = Some(id);
            }
            Some(UIRequest::Quit) => break,
            None => break,
        }
    }
}
