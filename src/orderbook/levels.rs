use std::{cmp::Ordering, hash::{Hash, Hasher}};

use crate::exchanges::ExchangeType;

/// A representation of bid levels that can be ordered according to the best deal.
#[derive(Debug, Default, Copy, Clone)]
pub struct BidLevel {
    pub price: f64,
    pub amount: f64,
    pub exchange: ExchangeType,
}

impl Hash for BidLevel {
    fn hash<H: Hasher>(&self, state: &mut H) {
        format!("{}|{}|{}", self.price, self.amount, self.exchange.to_string()).hash(state)
    }
}

impl Ord for BidLevel {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.price / self.amount).total_cmp(&(other.price / other.amount))
    }
}

impl PartialOrd for BidLevel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for BidLevel {
    fn eq(&self, other: &Self) -> bool {
        self.price / self.amount == other.price / other.amount && self.exchange == other.exchange
    }
}

impl Eq for BidLevel {}

/// A representation of ask levels that can be ordered according to the best deal.
#[derive(Debug, Default, Copy, Clone)]
pub struct AskLevel {
    pub price: f64,
    pub amount: f64,
    pub exchange: ExchangeType,
}

impl Hash for AskLevel {
    fn hash<H: Hasher>(&self, state: &mut H) {
        format!("{}|{}|{}", self.price, self.amount, self.exchange.to_string()).hash(state)
    }
}

impl Ord for AskLevel {
    fn cmp(&self, other: &Self) -> Ordering {
        (self.amount / self.price).total_cmp(&(other.amount / other.price))
    }
}

impl PartialOrd for AskLevel {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl PartialEq for AskLevel {
    fn eq(&self, other: &Self) -> bool {
        self.amount / self.price == other.amount / other.price && self.exchange == other.exchange
    }
}

impl Eq for AskLevel {}
