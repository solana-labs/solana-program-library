use {
    serde::{Deserialize, Serialize},
    serde_json::Value,
    std::collections::HashMap,
};

#[derive(Deserialize, Serialize, Debug, Clone)]
pub struct GitToken {
    #[serde(rename = "chainId")]
    pub chain_id: i32,
    pub address: String,
    pub symbol: String,
    pub name: String,
    pub decimals: i32,
    #[serde(rename = "logoURI", default)]
    pub logo_uri: String,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(flatten)]
    pub extra: HashMap<String, Value>,
}
