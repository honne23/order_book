use std::{collections::HashMap, error::Error};
use tokio_stream::StreamMap;

use crate::exchanges::{
    binance::Binance, bitstamp::Bitstamp, Exchange, ExchangeType,
};

use super::Orderbook;



pub trait OrderbookBuilderState {}
pub struct Empty {}
impl OrderbookBuilderState for Empty {}

pub struct WithMaxDepth {}
impl OrderbookBuilderState for WithMaxDepth {}
pub struct WithSymbol {}
impl OrderbookBuilderState for WithSymbol{}
pub struct WithExchange {}
impl OrderbookBuilderState for WithExchange{}

pub struct OrderbookBuilder<State: OrderbookBuilderState> {
    exchanges: HashMap<ExchangeType, Box<dyn Exchange + Send + Sync>>,
    symbol: String,
    max_depth: usize,
    state: std::marker::PhantomData<State>
}

impl<S: OrderbookBuilderState> OrderbookBuilder<S> {
    pub fn new() -> OrderbookBuilder<Empty> {
        OrderbookBuilder {
            exchanges: HashMap::new(),
            symbol: String::new(),
            max_depth: 0,
            state: std::marker::PhantomData
        }
    }

    
    pub fn with_max_depth(self, max_depth: usize) -> OrderbookBuilder<WithMaxDepth> {
        OrderbookBuilder { exchanges: self.exchanges, symbol: self.symbol, max_depth: max_depth, state: std::marker::PhantomData }
    }


}

impl OrderbookBuilder<WithMaxDepth> {
    pub fn with_symbol<Sym: Into<String>>(self, symbol: Sym) -> OrderbookBuilder<WithSymbol> {
        OrderbookBuilder { exchanges: self.exchanges, symbol: symbol.into(), max_depth: self.max_depth, state: std::marker::PhantomData }
    }
}

impl OrderbookBuilder<WithSymbol> {
    pub fn with_exchange(mut self, exchange: ExchangeType) -> OrderbookBuilder<WithExchange> {
        match exchange {
            ExchangeType::Bitstamp => self.exchanges.insert(exchange, Box::new(Bitstamp {})),
            ExchangeType::Binance => self.exchanges.insert(exchange, Box::new(Binance {})),
            _ => None, // throw not supported error
        };
        OrderbookBuilder { exchanges: self.exchanges, symbol: self.symbol, max_depth: self.max_depth, state: std::marker::PhantomData }
    }
}

impl OrderbookBuilder<WithExchange> {
    pub fn with_exchange(&mut self, exchange: ExchangeType) -> &mut Self {
        match exchange {
            ExchangeType::Bitstamp => self.exchanges.insert(exchange, Box::new(Bitstamp {})),
            ExchangeType::Binance => self.exchanges.insert(exchange, Box::new(Binance {})),
            _ => None, // throw not supported error
        };
        self
    }
    
    pub async fn build(&self) -> Result<Orderbook, Box<dyn Error>> {
        let mut exchange_streams = StreamMap::new();
        for (name, exchange) in &self.exchanges {
            exchange_streams.insert(
                *name,
                exchange.connect(self.symbol.clone()).await?,
            );
        }
        Ok(Orderbook {
            exchange_streams,
            max_depth: self.max_depth,
        })
    }
}