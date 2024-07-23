use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(tag = "type")]
#[serde(rename_all = "camelCase")]
pub enum Actions {
    Order(Order),
    Cancel(Cancel),
}

#[derive(Deserialize, Serialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Order {
    pub is_buy: bool,
    pub limit_px: u64,
    pub sz: u64,
    pub oid: u64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Cancel {
    pub oid: u64,
}
