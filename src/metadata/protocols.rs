use anyhow::{anyhow, Result};
use once_cell::sync::Lazy;
use reqwest::{header::AUTHORIZATION, Client};
use serde::{Deserialize, Serialize};

use crate::core::Enso;

pub static ENSO_PROTOCOL: Lazy<Protocol> = Lazy::new(|| Protocol {
    slug: "enso".to_string(),
    url: "https://api.enso.finance".to_string(),
});

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Protocol {
    pub slug: String,
    pub url: String,
}

impl Enso {
    pub async fn get_protocols(&self) -> Result<Vec<Protocol>> {
        let client = Client::new();
        let url = format!("{}/protocols", self.get_api_url());
        let auth = format!("Bearer {}", self.api_key);
        let response = client.get(&url).header(AUTHORIZATION, auth).send().await;
        response
            .map_err(|_| anyhow!("Couldn't get tokens"))?
            .json::<Vec<Protocol>>()
            .await
            .map_err(|_| anyhow!("Couldn't parse result"))
    }
}

#[cfg(test)]
mod test {
    use crate::core::Version;

    use super::*;

    #[tokio::test]
    async fn test_get_protocols() {
        let enso = Enso::new(
            "1e02632d-6feb-4a75-a157-documentation".to_string(),
            Version::V1,
        );

        let projects = enso.get_protocols().await;

        assert!(projects.is_ok());
    }
}
