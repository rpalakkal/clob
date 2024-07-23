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

impl Order {
    pub fn new(is_buy: bool, limit_px: u64, sz: u64, oid: u64) -> Self {
        Self {
            is_buy,
            limit_px,
            sz,
            oid,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Cancel {
    pub oid: u64,
}
