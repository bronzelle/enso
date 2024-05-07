use tokio;

use enso::core::{Enso, Version};

mod config;

#[tokio::main]
async fn main() {
    let config = config::Config::default();
    let enso = Enso::new(config.api_key, Version::V1);
    let tokens = enso.get_tokens().await;
    println!("{:?}", tokens);
}
