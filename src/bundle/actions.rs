use anyhow::{anyhow, Result};
use reqwest::{header::AUTHORIZATION, Client};
use serde::Deserialize;

use crate::core::Enso;

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Action {
    pub action: String,
    #[serde(with = "object_as_vector")]
    pub inputs: Vec<(String, String)>,
}

impl Enso {
    pub async fn get_actions(&self) -> Result<Vec<Action>> {
        let client = Client::new();
        let url = format!("{}/actions", self.get_api_url());
        let auth = format!("Bearer {}", self.api_key);
        let response = client.get(&url).header(AUTHORIZATION, auth).send().await;
        response
            .map_err(|_| anyhow!("Couldn't get tokens"))?
            .json::<Vec<Action>>()
            .await
            .map_err(|_| anyhow!("Couldn't parse result"))
    }
}

mod object_as_vector {
    use serde::de::Error;
    use serde::Deserializer;
    use serde_json::Value;

    pub fn deserialize<'de, D>(des: D) -> Result<Vec<(String, String)>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let object: Value = serde::Deserialize::deserialize(des)?;
        let Value::Object(fields) = object else {
            return Err(D::Error::custom(""));
        };

        Ok(fields
            .into_iter()
            .map(|(f, v)| {
                (
                    f,
                    match v {
                        Value::String(v) => v.to_owned(),
                        _ => "".to_owned(),
                    },
                )
            })
            .collect())
    }
}

#[cfg(test)]
mod test {
    use crate::core::Version;

    use super::*;

    #[tokio::test]
    async fn test_get_actions() {
        let enso = Enso::new(
            "1e02632d-6feb-4a75-a157-documentation".to_string(),
            Version::V1,
        );

        let actions = enso.get_actions().await;

        assert!(actions.is_ok());
    }
}
