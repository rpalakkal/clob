use alloy::primitives::Address;
use serde::{Deserialize, Serialize};

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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct FillStatus {
    pub oid: u64,
    pub sz: u64,
    pub addr: Address,
    pub filled_sz: u64,
    pub fills: Vec<OrderFill>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct OrderFill {
    pub maker_oid: u64,
    pub taker_oid: u64,
    pub sz: u64,
}

impl OrderFill {
    pub fn new(maker_oid: u64, taker_oid: u64, sz: u64) -> Self {
        Self {
            maker_oid,
            taker_oid,
            sz,
        }
    }
}
