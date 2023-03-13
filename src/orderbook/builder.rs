use std::{collections::HashMap, error::Error};
use tokio_stream::StreamMap;

use crate::exchanges::{binance::Binance, bitstamp::Bitstamp, Exchange, ExchangeType};

use super::Orderbook;

pub trait OrderbookBuilderState {}

/// The builder's initial state
pub struct Empty {}
impl OrderbookBuilderState for Empty {}
/// The builder must provide a `max_depth` first
pub struct WithMaxDepth {}
impl OrderbookBuilderState for WithMaxDepth {}

/// The builder must provide a `symbol` second
pub struct WithSymbol {}
impl OrderbookBuilderState for WithSymbol {}

/// The builder must finally provide a collection of `ExchangeType` before calling `.build()`
pub struct WithExchange {}
impl OrderbookBuilderState for WithExchange {}

/// A builder that implements the Typestate pattern to construct Orderbooks.
pub struct OrderbookBuilder<State: OrderbookBuilderState> {
    exchanges: HashMap<ExchangeType, Box<dyn Exchange + Send + Sync>>,
    symbol: String,
    max_depth: usize,
    state: std::marker::PhantomData<State>,
}

impl<S: OrderbookBuilderState> OrderbookBuilder<S> {
    pub fn new() -> OrderbookBuilder<Empty> {
        OrderbookBuilder {
            exchanges: HashMap::new(),
            symbol: String::new(),
            max_depth: 0,
            state: std::marker::PhantomData,
        }
    }

    pub fn with_max_depth(self, max_depth: usize) -> OrderbookBuilder<WithMaxDepth> {
        OrderbookBuilder {
            exchanges: self.exchanges,
            symbol: self.symbol,
            max_depth,
            state: std::marker::PhantomData,
        }
    }
}

impl OrderbookBuilder<WithMaxDepth> {
    pub fn with_symbol<Sym: Into<String>>(self, symbol: Sym) -> OrderbookBuilder<WithSymbol> {
        OrderbookBuilder {
            exchanges: self.exchanges,
            symbol: symbol.into(),
            max_depth: self.max_depth,
            state: std::marker::PhantomData,
        }
    }
}

impl OrderbookBuilder<WithSymbol> {
    pub fn with_exchanges(mut self, exchanges: &[ExchangeType]) -> OrderbookBuilder<WithExchange> {
        exchanges.iter().for_each(|exchange| {
            match exchange {
                ExchangeType::Bitstamp => self.exchanges.insert(*exchange, Box::new(Bitstamp {})),
                ExchangeType::Binance => self.exchanges.insert(*exchange, Box::new(Binance {})),
                _ => None, // throw not supported error
            };
        });

        OrderbookBuilder {
            exchanges: self.exchanges,
            symbol: self.symbol,
            max_depth: self.max_depth,
            state: std::marker::PhantomData,
        }
    }
}

impl OrderbookBuilder<WithExchange> {
    /// Can only be called on fully constructed OrderbookBuilder.
    ///
    /// Returns an `Orderbook` where you can call `.collect()` to start streaming events
    pub async fn build<T: Orderbook>(self) -> Result<T, Box<dyn Error>> {
        let mut exchange_streams = StreamMap::new();
        for (name, exchange) in &self.exchanges {
            exchange_streams.insert(*name, exchange.connect(self.symbol.clone()).await?);
        }
        Ok(T::new(self.max_depth, exchange_streams))
    }
}
