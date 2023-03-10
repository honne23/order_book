use std::{error::Error, collections::BinaryHeap, cmp::Reverse, mem};
use async_stream::stream;
use futures::Stream;
use tokio_stream::{StreamMap, StreamExt};

use crate::exchanges::{Exchange, ExchangeType, bitstamp::Bitstamp, binance::Binance, SnapshotStream};

use self::levels::{BidLevel, AskLevel};
mod levels;

pub struct Orderbook {
    max_depth: usize,
    exchange_streams: StreamMap<ExchangeType, SnapshotStream>
}

pub struct OrderbookBuilder {
    exchanges: Vec<Box<dyn Exchange>>,
    symbol: String,
    max_depth: usize,
}

impl OrderbookBuilder {
    pub fn new() -> Self {
            Self { exchanges: Vec::new(), symbol: String::new(), max_depth: 0 }
    }

    pub fn with_exchange(&mut self, exchange: ExchangeType) -> &mut Self {
        match exchange {
            ExchangeType::Bitstamp => self.exchanges.push(Box::new(Bitstamp{})),
            ExchangeType::Binance => self.exchanges.push(Box::new(Binance{})),
            _ => ()
        };
        self
    }

    pub fn with_max_depth(&mut self, max_depth: usize) -> &mut Self {
        self.max_depth = max_depth;
        self
    }

    pub fn with_symbol<S: Into<String>>(&mut self, symbol: S) -> &mut Self {
        self.symbol = symbol.into();
        self
    }

    pub async fn build(&self) -> Result<Orderbook, Box<dyn Error>> {
        let mut exchange_streams = StreamMap::new();
        for exchange in &self.exchanges {
            exchange_streams.insert(exchange.name(), exchange.connect(self.symbol.clone()).await?);
        }
        Ok(Orderbook { exchange_streams, max_depth: self.max_depth })
    }
}

impl Orderbook {
    pub async fn collect(&mut self) -> impl Stream<Item= (Vec<BidLevel>, Vec<AskLevel>)> + '_ {
        let event_stream = stream! {
            let bid_heap = &mut BinaryHeap::<Reverse<BidLevel>>::with_capacity(self.max_depth + 1);
            let ask_heap = &mut BinaryHeap::<Reverse<AskLevel>>::with_capacity(self.max_depth + 1);
            let mut round = 0;
            loop {
                let (key, val) = self.exchange_streams.next().await.unwrap();
                let snapshot = val.unwrap();
                snapshot.bids.iter().for_each(|bid|{
                    bid_heap.push(Reverse(BidLevel {
                        price: bid[0],
                        amount: bid[1],
                        exchange: key,
                    }));
                    if bid_heap.len() > self.max_depth {
                        bid_heap.pop();
                    }
                });

                snapshot.asks.iter().for_each(|ask| {
                    ask_heap.push(Reverse(AskLevel {
                        price: ask[0],
                        amount: ask[1],
                        exchange: key,
                    }));
                    if ask_heap.len() > self.max_depth {
                        ask_heap.pop();
                    }
                });
                round +=1;
                if round >= self.exchange_streams.len() && round % self.exchange_streams.len() == 0 {
                    yield (
                        bid_heap.clone()
                            .into_sorted_vec()
                            .iter_mut()
                            .map(|x| mem::take(&mut x.0))
                            .collect::<Vec<BidLevel>>(),
                        ask_heap.clone()
                            .into_sorted_vec()
                            .iter_mut()
                            .map(|x| mem::take(&mut x.0))
                            .collect::<Vec<AskLevel>>(),
                    );
                }
                
            }
        };
        event_stream
        
    }
}