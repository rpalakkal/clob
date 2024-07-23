use std::{collections::HashMap, sync::Arc};

use alloy::primitives::Address;
use axum::{extract::State, Json};
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;

use crate::{
    order::{FillStatus, Order},
    orderbook::OrderBook,
};

#[derive(Clone, Serialize, Deserialize)]
pub struct UserBalance {
    pub a: u64,
    pub b: u64,
}

#[derive(Clone, Default)]
pub struct SharedState {
    balances: Arc<Mutex<HashMap<Address, UserBalance>>>,
    book: Arc<Mutex<OrderBook>>,
    order_status: Arc<Mutex<HashMap<u64, FillStatus>>>,
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
    status: Option<FillStatus>,
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

#[derive(Clone, Debug, Deserialize)]
pub struct OrderStatusRequest {
    oid: u64,
}

#[derive(Clone, Debug, Serialize)]
pub struct OrderStatusResponse {
    status: Option<FillStatus>,
}

pub async fn server(state: SharedState) {
    let app = axum::Router::new()
        .route("/deposit", axum::routing::post(deposit))
        .route("/withdraw", axum::routing::post(withdraw))
        .route("/orders", axum::routing::post(place_order))
        .route("/cancel", axum::routing::post(cancel))
        .route("/status", axum::routing::post(order_status))
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
            status: None,
        });
    }
    let mut oid = state.oid.lock().await;
    let order = req.to_order(*oid);
    let order_id = order.oid;
    *oid += 1;
    let (remaining_amount, fills) = state.book.lock().await.limit(order);
    let fill_sz = req.sz - remaining_amount;
    if req.is_buy {
        balance.b -= req.sz;
        balance.a += fill_sz;
    } else {
        balance.a -= req.sz;
        balance.b += fill_sz;
    }
    let mut order_status = state.order_status.lock().await;
    for fill in fills.iter() {
        let maker_order_status = order_status.get_mut(&fill.maker_oid).unwrap();
        maker_order_status.filled_sz += fill.sz;
        maker_order_status.fills.push(fill.clone());
        if req.is_buy {
            balances.get_mut(&maker_order_status.addr).unwrap().b += fill.sz;
        } else {
            balances.get_mut(&maker_order_status.addr).unwrap().a += fill.sz;
        }
    }
    let fill_status = FillStatus {
        oid: order_id,
        sz: req.sz,
        filled_sz: fill_sz,
        fills,
        addr: req.addr,
    };
    order_status.insert(order_id, fill_status.clone());

    Json(PlaceOrderResponse {
        success: true,
        status: Some(fill_status),
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

async fn order_status(
    State(state): State<SharedState>,
    Json(req): Json<OrderStatusRequest>,
) -> Json<OrderStatusResponse> {
    let order_status = state.order_status.lock().await;
    let status = order_status.get(&req.oid).cloned();
    Json(OrderStatusResponse { status })
}
