use anyhow::{anyhow, Result};
use reqwest::header::AUTHORIZATION;
use reqwest::Client;
use serde_json::{Map, Number, Value};

use crate::core::Enso;
use crate::metadata::protocols::{Protocol, ENSO_PROTOCOL};

use super::actions::{Action, ACTION_CALL};

#[derive(Clone, Debug)]
pub enum ParamValue {
    Value(String),
    LastTransaction,
    Transaction(usize),
    ValueArray(Vec<ParamValue>),
}

struct Transaction {
    protocol: Protocol,
    action: Action,
    args: Vec<ParamValue>,
}

pub struct Bundle {
    chain_id: u32,
    transactions: Vec<Transaction>,
}

impl Bundle {
    pub fn new(chain_id: u32) -> Bundle {
        Bundle {
            chain_id,
            transactions: Default::default(),
        }
    }

    pub fn add_enso_action(&mut self, action: Action, args: Vec<ParamValue>) {
        self.add_action(ENSO_PROTOCOL.clone(), action, args);
    }

    pub fn add_action(&mut self, protocol: Protocol, action: Action, args: Vec<ParamValue>) {
        self.transactions.push(Transaction {
            protocol,
            action,
            args,
        });
    }

    pub fn add_call(&mut self, mut args: Vec<ParamValue>, abi_args: Vec<ParamValue>) {
        args.push(ParamValue::ValueArray(abi_args));
        self.add_action(ENSO_PROTOCOL.clone(), ACTION_CALL.clone(), args);
    }

    fn to_json(&self) -> String {
        fn output_of_call_at(tx: usize) -> Value {
            let mut object = Map::new();
            object.insert(
                "useOutputOfCallAt".to_owned(),
                Value::Number(Number::from(tx)),
            );
            Value::Object(object)
        }

        fn param_value_to_json(value: &ParamValue, current_tx: usize) -> Value {
            match value {
                ParamValue::Value(v) => Value::String(v.clone()),
                ParamValue::LastTransaction => {
                    if current_tx > 0 {
                        output_of_call_at(current_tx - 1)
                    } else {
                        Value::String("0".to_owned())
                    }
                }
                ParamValue::Transaction(t) => output_of_call_at(*t),
                ParamValue::ValueArray(values) => {
                    let mut array = Vec::new();
                    for value in values {
                        array.push(param_value_to_json(value, current_tx));
                    }
                    Value::Array(array)
                }
            }
        }

        // let mut bundle = Value::Array(Vec::new());
        let mut bundle = Vec::new();
        for (current_tx, transaction) in self.transactions.iter().enumerate() {
            let mut tx = Map::new();
            tx.insert(
                "protocol".to_owned(),
                Value::String(transaction.protocol.slug.clone()),
            );
            tx.insert(
                "action".to_owned(),
                Value::String(transaction.action.action.clone()),
            );
            let mut args = Map::new();
            for ((name, _), value) in transaction
                .action
                .inputs
                .iter()
                .zip(transaction.args.iter())
            {
                args.insert(name.clone(), param_value_to_json(value, current_tx));
            }
            tx.insert("args".to_owned(), Value::Object(args));
            bundle.push(tx);
        }

        serde_json::to_string(&bundle).unwrap()
    }
}

impl Enso {
    pub async fn send_bundle(&self, bundle: Bundle, from_address: &str) -> Result<()> {
        let client = Client::new();
        let url = format!("{}/shortcuts/bundle", self.get_api_url());
        let auth = format!("Bearer {}", self.api_key);
        let query = vec![
            ("chainId", bundle.chain_id.to_string()),
            ("fromAddress", from_address.to_owned()),
        ];
        let response = client
            .post(&url)
            .header(AUTHORIZATION, auth)
            .query(&query)
            .json(&bundle.to_json())
            .send()
            .await;
        let _ = response.map_err(|_| anyhow!("Couldn't send transaction"))?;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use once_cell::sync::Lazy;

    use crate::core::Version;

    use super::*;

    static ACTION_ROUTE: Lazy<Action> = Lazy::new(|| Action {
        action: "route".to_owned(),
        inputs: vec![
            ("amountIn".to_owned(), "Raw amount to sell".to_owned()),
            ("slippage".to_owned(), "Amount of slippage".to_owned()),
            ("tokenIn".to_owned(), "Address of token to sell".to_owned()),
            ("tokenOut".to_owned(), "Address of token to buy".to_owned()),
        ],
    });

    static JSON: &'static str = r#"
    [
        {
            "protocol": "enso",
            "action": "route",
            "args": {
            "tokenIn": "0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee",
            "tokenOut": "0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84",
            "amountIn": "100000000000",
            "slippage": "300"
            }
        },
        {
            "protocol": "enso",
            "action": "call",
            "args": {
            "address": "0xCc9EE9483f662091a1de4795249E24aC0aC2630f",
            "method": "transfer",
            "abi": "function transfer(address,uint256) external",
            "args": [
                "0x93621DCA56fE26Cdee86e4F6B18E116e9758Ff11",
                {
                "useOutputOfCallAt": 1
                }
            ]
            }
        }
    ]
    "#;

    fn create_bundle(chain_id: u32) -> Bundle {
        let mut bundle = Bundle::new(chain_id);
        bundle.add_enso_action(
            ACTION_ROUTE.clone(),
            vec![
                ParamValue::Value("100000000000".to_string()),
                ParamValue::Value("300".to_string()),
                ParamValue::Value("0xeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeeee".to_string()),
                ParamValue::Value("0xae7ab96520DE3A18E5e111B5EaAb095312D7fE84".to_string()),
            ],
        );
        bundle.add_call(
            vec![
                ParamValue::Value("0xCc9EE9483f662091a1de4795249E24aC0aC2630f".to_string()),
                ParamValue::Value("transfer".to_string()),
                ParamValue::Value("function transfer(address,uint256) external".to_string()),
            ],
            vec![
                ParamValue::Value("0x93621DCA56fE26Cdee86e4F6B18E116e9758Ff11".to_string()),
                ParamValue::Transaction(1),
            ],
        );
        bundle
    }

    #[test]
    fn test_create_bundle() {
        let bundle = create_bundle(1);

        let original: Value = serde_json::from_str(JSON).unwrap();
        let bundle: Value = serde_json::from_str(&bundle.to_json()).unwrap();

        assert_eq!(bundle, original);
    }

    #[tokio::test]
    async fn test_send_bundle() {
        let enso = Enso::new(
            "1e02632d-6feb-4a75-a157-documentation".to_string(),
            Version::V1,
        );
        let bundle = create_bundle(1);
        let from_address = "0xd8da6bf26964af9d7eed9e03e53415d37aa96045";
        let result = enso.send_bundle(bundle, from_address).await;

        assert!(result.is_ok());
    }
}
