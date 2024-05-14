use std::{pin::Pin, task::Poll};

use anyhow::{anyhow, Result};
use futures::{Future, Stream};
use reqwest::{header::AUTHORIZATION, Client, Response};
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

#[derive(Copy, Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Meta {
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

type ServerOutput = Result<Response, reqwest::Error>;
type ParsingOutput = Result<Tokens, reqwest::Error>;

enum StreamStates {
    Checking,
    PollingServer(Option<Pin<Box<dyn Future<Output = ServerOutput> + Send>>>),
    PollingParsing(Option<Pin<Box<dyn Future<Output = ParsingOutput> + Send>>>),
}

pub struct PaginatedTokensStream {
    client: Client,
    url: String,
    auth: String,
    params: Vec<(String, String)>,
    page: u32,
    total_pages: Option<u32>,
    state: StreamStates,
}

impl Stream for PaginatedTokensStream {
    type Item = Result<Vec<String>>;

    fn poll_next(
        self: std::pin::Pin<&mut Self>,
        cx: &mut std::task::Context,
    ) -> std::task::Poll<Option<Self::Item>> {
        let this = self.get_mut();

        if let StreamStates::Checking = this.state {
            if this
                .total_pages
                .is_some_and(|total_pages| this.page >= total_pages)
            {
                return Poll::Ready(None);
            }
            let response = this
                .client
                .get(&this.url)
                .header(AUTHORIZATION, this.auth.clone())
                .query(&this.params)
                .query(&[("page".to_string(), (this.page + 1).to_string())])
                .send();
            this.state = StreamStates::PollingServer(Some(Box::pin(response)))
        }

        if let StreamStates::PollingServer(server) = &mut this.state {
            let Some(future) = server.as_mut() else {
                return Poll::Ready(None);
            };
            let response = match futures::ready!(future.as_mut().poll(cx)) {
                Ok(response) => response,
                Err(_) => return Poll::Ready(Some(Err(anyhow!("Couldn't get tokens")))),
            };
            let tokens = response.json::<Tokens>();
            this.state = StreamStates::PollingParsing(Some(Box::pin(tokens)))
        }

        if let StreamStates::PollingParsing(parsing) = &mut this.state {
            let Some(future) = parsing.as_mut() else {
                this.state = StreamStates::Checking;
                return Poll::Ready(None);
            };
            match futures::ready!(future.as_mut().poll(cx)) {
                Ok(tokens) => {
                    this.page += 1;
                    this.total_pages = Some(tokens.meta.last_page);
                    this.state = StreamStates::Checking;
                    return std::task::Poll::Ready(Some(Ok(tokens
                        .data
                        .iter()
                        .map(|token| token.address.clone())
                        .collect())));
                }
                Err(_) => {
                    this.state = StreamStates::Checking;
                    return std::task::Poll::Ready(Some(Err(anyhow!("Couldn't parse result"))));
                }
            }
        };

        this.state = StreamStates::Checking;
        Poll::Ready(None)
    }
}

impl Enso {
    pub fn tokens_stream(
        &self,
        params: &[(&str, &str)],
    ) -> Pin<Box<dyn Stream<Item = Result<Vec<String>>> + Send>> {
        let client = Client::new();
        let url = format!("{}/tokens", self.get_api_url());
        let auth = format!("Bearer {}", self.api_key);
        let stream = PaginatedTokensStream {
            client,
            url,
            auth,
            params: params
                .iter()
                .map(|(f, v)| (f.to_string(), v.to_string()))
                .collect::<Vec<(String, String)>>(),
            page: 0,
            total_pages: None,
            state: StreamStates::Checking,
        };
        Box::pin(stream)
    }

    pub async fn get_tokens(&self, params: &[(&str, &str)]) -> Result<(Meta, Vec<String>)> {
        let client = Client::new();
        let url = format!("{}/tokens", self.get_api_url());
        let auth = format!("Bearer {}", self.api_key);
        let response = client
            .get(&url)
            .header(AUTHORIZATION, auth)
            .query(params)
            .send()
            .await;
        response
            .map_err(|_| anyhow!("Couldn't get tokens"))?
            .json::<Tokens>()
            .await
            .map_err(|_| anyhow!("Couldn't parse result"))
            .map(|tokens| {
                (
                    tokens.meta,
                    tokens
                        .data
                        .iter()
                        .map(|token| token.address.clone())
                        .collect(),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use futures::StreamExt;

    use crate::core::Version;

    use super::*;

    #[tokio::test]
    async fn test_get_tokens() {
        let enso = Enso::new(
            "1e02632d-6feb-4a75-a157-documentation".to_string(),
            Version::V1,
        );
        let filter = vec![("chainId", "10")];
        let mut page = 1;
        let mut total = 0u32;
        let tokens = enso.get_tokens(&filter).await;
        let Ok((meta, tokens)) = tokens else {
            panic!("tokens retrieving failed!");
        };
        total += tokens.len() as u32;
        page += 1;
        while page <= meta.last_page {
            let mut new_filter = filter.clone();
            let str_page = page.to_string();
            new_filter.push(("page", &str_page));
            let tokens = enso.get_tokens(&new_filter).await;
            let Ok((_, tokens)) = tokens else {
                panic!("retrieving tokens failed!");
            };
            total += tokens.len() as u32;
            page += 1;
        }
        assert_eq!(total, meta.total);
    }

    #[tokio::test]
    async fn test_tokens_stream() {
        let enso = Enso::new(
            "1e02632d-6feb-4a75-a157-documentation".to_string(),
            Version::V1,
        );
        let tokens = enso.get_tokens(&[("chainId", "10")]).await;
        let Ok((meta, _)) = tokens else {
            panic!("retrieving tokens failed!");
        };

        let mut total = 0u32;
        let mut tokens_streams = enso.tokens_stream(&[("chainId", "10")]);
        while let Some(tokens) = tokens_streams.next().await {
            let Ok(tokens) = tokens else {
                panic!("retrieving tokens failed!");
            };
            total += tokens.len() as u32;
        }
        assert_eq!(total, meta.total);
    }
}
