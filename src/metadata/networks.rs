use anyhow::{anyhow, Result};
use reqwest::{header::AUTHORIZATION, Client};
use serde::{Deserialize, Serialize};

use crate::core::Enso;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Network {
    pub id: u32,
    pub name: String,
}

impl Enso {
    /// Retrieves a list of available networks from the Enso API.
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `Network` instances or an error.
    pub async fn get_networks(&self) -> Result<Vec<Network>> {
        let client = Client::new();
        let url = format!("{}/networks", self.get_api_url());
        let auth = format!("Bearer {}", self.api_key);
        let response = client.get(&url).header(AUTHORIZATION, auth).send().await;
        response
            .map_err(|_| anyhow!("Couldn't get tokens"))?
            .json::<Vec<Network>>()
            .await
            .map_err(|_| anyhow!("Couldn't parse result"))
    }
}

#[cfg(test)]
mod test {
    use crate::core::Version;

    use super::*;

    #[tokio::test]
    async fn test_get_networks() {
        let enso = Enso::new(
            "1e02632d-6feb-4a75-a157-documentation".to_string(),
            Version::V1,
        );

        let networks = enso.get_networks().await;

        assert!(networks.is_ok());
    }
}
