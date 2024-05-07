use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
    pub api_key: String,
}

impl Default for Config {
    fn default() -> Self {
        dotenv::dotenv().ok();
        let Ok(config) = envy::from_env::<Config>() else {
            panic!("Missing env variables");
        };
        config
    }
}
