use std::collections::{BTreeMap, HashMap};

use eyre::OptionExt;

use crate::order::Order;

#[derive(Clone, Debug, Default)]
pub struct OrderBook {
    pub bids: BTreeMap<u64, Vec<Order>>,
    pub asks: BTreeMap<u64, Vec<Order>>,
    pub oid_to_level: HashMap<u64, u64>,
}

fn fill_at_price_level(level: &mut Vec<Order>, amount: u64) -> (u64, Vec<Order>) {
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
    let fills = level.drain(..complete_fills).collect();

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
            let level = self.bids.entry(order.limit_px).or_insert(Vec::new());
            level.push(order);
        } else {
            let level = self.asks.entry(order.limit_px).or_insert(Vec::new());
            level.push(order);
        }
    }

    pub fn limit(&mut self, order: Order) -> eyre::Result<Vec<Order>> {
        let mut remaining_amount = order.sz;
        let mut ask_min = self.ask_min();
        let mut bid_max = self.bid_max();
        let mut fills = vec![];
        if order.is_buy {
            if order.limit_px >= ask_min {
                while remaining_amount > 0 && order.limit_px >= ask_min {
                    let mut level = self.asks.get_mut(&ask_min).unwrap();
                    let (new_remaining_amount, new_fills) =
                        fill_at_price_level(&mut level, remaining_amount);
                    remaining_amount = new_remaining_amount;
                    fills.extend(new_fills);
                    if level.is_empty() {
                        self.asks.remove(&ask_min);
                    }
                    if remaining_amount > 0 {
                        ask_min = self.ask_min();
                    }
                }
            }
        } else {
            if order.limit_px <= bid_max {
                while remaining_amount > 0 && order.limit_px <= bid_max {
                    let mut level = self.bids.get_mut(&bid_max).unwrap();
                    let (new_remaining_amount, new_fills) =
                        fill_at_price_level(&mut level, remaining_amount);
                    remaining_amount = new_remaining_amount;
                    fills.extend(new_fills);
                    if level.is_empty() {
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

    pub fn cancel(&mut self, oid: u64) -> eyre::Result<()> {
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

#[cfg(test)]
mod tests {
    use super::*;

    macro_rules! assert_bid_ask {
        ($book:expr, $expected_bid:expr, $expected_ask:expr) => {
            assert_eq!($book.bid_max(), $expected_bid);
            assert_eq!($book.ask_min(), $expected_ask);
        };
    }

    #[test]
    fn test_bid_max() {
        let mut book = OrderBook::default();
        book.limit(Order::new(true, 10, 10, 1)).unwrap();
        book.limit(Order::new(true, 20, 10, 2)).unwrap();
        book.limit(Order::new(true, 30, 10, 3)).unwrap();
        assert_bid_ask!(book, 30, u64::MAX);
    }

    #[test]
    fn test_ask_min() {
        let mut book = OrderBook::default();
        book.limit(Order::new(false, 10, 10, 1)).unwrap();
        book.limit(Order::new(false, 20, 10, 2)).unwrap();
        book.limit(Order::new(false, 30, 10, 3)).unwrap();
        assert_bid_ask!(book, 0, 10);
    }

    #[test]
    fn test_crossing_bid_max() {
        let mut book = OrderBook::default();
        book.limit(Order::new(true, 10, 10, 1)).unwrap();
        book.limit(Order::new(true, 20, 10, 2)).unwrap();
        book.limit(Order::new(true, 30, 10, 3)).unwrap();
        book.limit(Order::new(false, 25, 10, 5)).unwrap();
        assert_bid_ask!(book, 20, u64::MAX);
    }

    #[test]
    fn test_crossing_ask_min() {
        let mut book = OrderBook::default();
        book.limit(Order::new(false, 10, 10, 1)).unwrap();
        book.limit(Order::new(false, 20, 10, 2)).unwrap();
        book.limit(Order::new(false, 30, 10, 3)).unwrap();
        book.limit(Order::new(true, 25, 10, 5)).unwrap();
        assert_bid_ask!(book, 0, 20);
    }

    #[test]
    fn test_resting_bid_ask() {
        let mut book = OrderBook::default();
        book.limit(Order::new(true, 10, 10, 1)).unwrap();
        book.limit(Order::new(true, 20, 10, 2)).unwrap();
        book.limit(Order::new(false, 30, 10, 3)).unwrap();
        book.limit(Order::new(false, 25, 10, 5)).unwrap();
        assert_bid_ask!(book, 20, 25);
    }

    #[test]
    fn test_fill_at_price_level() {
        let mut level = vec![Order::new(true, 10, 10, 1), Order::new(true, 10, 10, 2)];
        let (remaining_amount, fills) = fill_at_price_level(&mut level, 10);
        assert_eq!(remaining_amount, 0);
        assert_eq!(fills.len(), 1);
        assert_eq!(fills[0].oid, 1);
    }
}
