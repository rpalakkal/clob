use std::{collections::BTreeMap, sync::Arc};

use alloy::primitives::Address;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{order::Order, orderbook::OrderBook};

#[derive(Clone, Serialize, Deserialize)]
pub struct UserBalance {
    pub a: u64,
    pub b: u64,
}

#[derive(Clone, Default)]
pub struct SharedState {
    balances: Arc<Mutex<BTreeMap<Address, UserBalance>>>,
    book: Arc<Mutex<OrderBook>>,
    oid: Arc<Mutex<u64>>,
}

#[derive(Clone, Deserialize)]
struct DepositRequest {
    addr: Address,
    amounts: UserBalance,
}

#[derive(Clone, Deserialize)]
struct WithdrawRequest {
    addr: Address,
    amounts: UserBalance,
}

#[derive(Clone, Deserialize)]
struct PlaceOrderRequest {
    addr: Address,
    is_buy: bool,
    limit_px: u64,
    sz: u64,
}

impl PlaceOrderRequest {
    fn to_order(&self, oid: u64) -> Order {
        Order {
            is_buy: self.is_buy,
            limit_px: self.limit_px,
            sz: self.sz,
            oid,
        }
    }
}

#[derive(Clone, Serialize)]
struct PlaceOrderResponse {
    success: bool,
    oid: Option<u64>,
}

#[derive(Clone, Serialize)]
struct WithdrawResponse {
    success: bool,
}

#[derive(Clone, Serialize)]
struct DepositResponse {
    success: bool,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CancelRequest {
    oid: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct CancelResponse {
    success: bool,
}

pub async fn server(state: SharedState) {
    let app = axum::Router::new()
        .route("/deposit", axum::routing::post(deposit))
        .route("/withdraw", axum::routing::post(withdraw))
        .route("/orders", axum::routing::post(place_order))
        .route("/cancel", axum::routing::post(cancel))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn deposit(
    State(state): State<SharedState>,
    Json(req): Json<DepositRequest>,
) -> Json<DepositResponse> {
    let mut balances = state.balances.lock().await;
    balances.insert(req.addr, req.amounts);
    Json(DepositResponse { success: true })
}

async fn withdraw(
    State(state): State<SharedState>,
    Json(req): Json<WithdrawRequest>,
) -> Json<WithdrawResponse> {
    let mut balances = state.balances.lock().await;
    let addr = req.addr;
    let balance = balances.get_mut(&addr).unwrap();
    let res = if balance.a < req.amounts.a || balance.b < req.amounts.b {
        WithdrawResponse { success: false }
    } else {
        balance.a -= req.amounts.a;
        balance.b -= req.amounts.b;
        WithdrawResponse { success: true }
    };
    Json(res)
}

async fn place_order(
    State(state): State<SharedState>,
    Json(req): Json<PlaceOrderRequest>,
) -> Json<PlaceOrderResponse> {
    let mut balances = state.balances.lock().await;
    let addr = req.addr;
    let balance = balances.get_mut(&addr).unwrap();
    if (req.is_buy && balance.b < req.sz) || (!req.is_buy && balance.a < req.sz) {
        return Json(PlaceOrderResponse {
            success: false,
            oid: None,
        });
    }
    let mut oid = state.oid.lock().await;
    let order = req.to_order(*oid);
    let order_id = order.oid;
    state.book.lock().await.limit(order);
    *oid += 1;
    if req.is_buy {
        balance.b -= req.sz;
    } else {
        balance.a -= req.sz;
    }
    Json(PlaceOrderResponse {
        success: true,
        oid: Some(order_id),
    })
}

async fn cancel(
    State(state): State<SharedState>,
    Json(req): Json<CancelRequest>,
) -> Json<CancelResponse> {
    let mut book = state.book.lock().await;
    let success = book.cancel(req.oid).is_ok();
    Json(CancelResponse { success })
}
