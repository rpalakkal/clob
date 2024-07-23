use std::collections::{BTreeMap, HashMap, VecDeque};

use eyre::OptionExt;

use crate::order::Order;

pub trait SplitFront<T> {
    fn split_front(&mut self, at: usize) -> VecDeque<T>;
}

impl<T> SplitFront<T> for VecDeque<T> {
    fn split_front(&mut self, at: usize) -> VecDeque<T> {
        let mut front = self.drain(..at).collect();
        std::mem::swap(self, &mut front);
        front
    }
}

#[derive(Clone, Debug, Default)]
pub struct OrderBook {
    pub bids: BTreeMap<u64, VecDeque<Order>>,
    pub asks: BTreeMap<u64, VecDeque<Order>>,
    pub oid_to_level: HashMap<u64, u64>,
}

fn fill_at_price_level(level: &mut VecDeque<Order>, amount: u64) -> (u64, VecDeque<Order>) {
    let mut complete_fills = 0;
    let mut remaining_amount = amount;
    for order in level.iter_mut() {
        if order.sz <= remaining_amount {
            complete_fills += 1;
            remaining_amount -= order.sz;
        } else {
            order.sz -= remaining_amount;
            remaining_amount = 0;
            break;
        }
    }
    let fills = level.split_front(complete_fills);

    (remaining_amount, fills)
}

impl OrderBook {
    pub fn bid_max(&self) -> u64 {
        if let Some(level) = self.bids.iter().next_back() {
            level.0.clone()
        } else {
            0
        }
    }

    pub fn ask_min(&self) -> u64 {
        if let Some(level) = self.asks.iter().next() {
            level.0.clone()
        } else {
            u64::MAX
        }
    }

    fn enqueue_order(&mut self, order: Order) {
        self.oid_to_level.insert(order.oid, order.limit_px);
        if order.is_buy {
            let level = self.bids.entry(order.limit_px).or_insert(VecDeque::new());
            level.push_back(order);
        } else {
            let level = self.asks.entry(order.limit_px).or_insert(VecDeque::new());
            level.push_back(order);
        }
    }

    pub async fn limit(&mut self, order: Order) -> eyre::Result<Vec<Order>> {
        let mut remaining_amount = order.sz;
        let mut ask_min = self.ask_min();
        let mut bid_max = self.bid_max();
        let mut fills = vec![];
        if order.is_buy {
            if order.limit_px > ask_min {
                while remaining_amount > 0 && ask_min <= order.limit_px {
                    let mut level = self.asks.get_mut(&ask_min).unwrap();
                    let (new_remaining_amount, new_fills) =
                        fill_at_price_level(&mut level, remaining_amount);
                    remaining_amount = new_remaining_amount;
                    fills.extend(new_fills);
                    if new_remaining_amount > 0 {
                        self.asks.remove(&ask_min);
                    }
                    if remaining_amount > 0 {
                        ask_min = self.ask_min();
                    }
                }
            }
        } else {
            if order.limit_px < bid_max {
                while remaining_amount > 0 && bid_max >= order.limit_px {
                    let mut level = self.bids.get_mut(&bid_max).unwrap();
                    let (new_remaining_amount, new_fills) =
                        fill_at_price_level(&mut level, remaining_amount);
                    remaining_amount = new_remaining_amount;
                    fills.extend(new_fills);
                    if new_remaining_amount > 0 {
                        self.bids.remove(&bid_max);
                    }
                    if remaining_amount > 0 {
                        bid_max = self.bid_max();
                    }
                }
            }
        }

        if remaining_amount > 0 {
            self.enqueue_order(order);
        } else {
            fills.push(order);
        }

        Ok(fills)
    }

    pub async fn cancel(&mut self, oid: u64) -> eyre::Result<()> {
        let level_price = self.oid_to_level.get(&oid).ok_or_eyre("oid not found")?;
        if self.bids.contains_key(level_price) {
            let level = self
                .bids
                .get_mut(level_price)
                .ok_or_eyre("level not found")?;
            level.retain(|order| order.oid != oid);
        } else if self.asks.contains_key(level_price) {
            let level = self
                .asks
                .get_mut(level_price)
                .ok_or_eyre("level not found")?;
            level.retain(|order| order.oid != oid);
        } else {
            return Err(eyre::eyre!("oid not found"));
        }
        self.oid_to_level.remove(&oid);
        Ok(())
    }
}
