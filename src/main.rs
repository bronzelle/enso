use futures::StreamExt;

use enso::core::{Enso, Version};

mod config;

#[tokio::main]
async fn main() {
    let config = config::Config::default();
    let enso = Enso::new(config.api_key, Version::V1);
    let mut tokens_streams = enso.tokens_stream(&[("chainId", "10")]);
    while let Some(tokens) = tokens_streams.next().await {
        match tokens {
            Ok(tokens) => println!("{:?}", tokens.len()),
            Err(e) => println!("{:?}", e),
        }
    }
}
