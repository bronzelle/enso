use anyhow::{anyhow, Result};
use reqwest::header::AUTHORIZATION;
use serde::{Deserialize, Serialize};

use crate::core::Enso;

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Token {
    chain_id: u32,
    address: String,
    #[serde(rename = "type")]
    kind: String,
    protocol_slug: String,
    underlying_tokens: Vec<String>,
    primary_address: String,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Meta {
    total: u32,
    last_page: u32,
    current_page: u32,
    per_page: u32,
    prev: Option<u32>,
    next: Option<u32>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Tokens {
    meta: Meta,
    data: Vec<Token>,
}

impl Enso {
    pub async fn get_tokens(&self) -> Result<Vec<String>> {
        let client = reqwest::Client::new();
        let url = format!("{}/tokens", self.get_api_url());
        let auth = format!("Bearer {}", self.api_key);
        let params = [("page", "2")];
        let response = client
            .get(&url)
            .header(AUTHORIZATION, auth)
            .query(&params)
            .send()
            .await;
        response
            .map_err(|_| anyhow!("Couldn't get tokens"))?
            .json::<Tokens>()
            .await
            .map_err(|_| anyhow!("Couldn't parse result"))
            .map(|tokens| {
                tokens
                    .data
                    .iter()
                    .map(|token| token.address.clone())
                    .collect()
            })
    }
}
