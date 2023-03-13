use async_stream::stream;
use futures::Stream;
use std::{cmp::Reverse, collections::BinaryHeap, error::Error, mem};
use thiserror::Error;
use tokio_stream::{StreamExt, StreamMap};

use crate::exchanges::{ExchangeType, SnapshotStream};

use super::{levels::{AskLevel, BidLevel}, Orderbook};



#[derive(Error, Debug)]
enum OrderbookError {
    #[error("a stream has unexpectedly closed")]
    StreamCancelled,
}

pub struct HeapedBook {
    max_depth: usize,
    exchange_streams: StreamMap<ExchangeType, SnapshotStream>,
}

impl Orderbook for HeapedBook {
    type AskOrder = AskLevel;
    type BidOrder = BidLevel;

    fn new(max_depth: usize, exchanges: StreamMap<ExchangeType, SnapshotStream>) -> Self {
        Self { max_depth, exchange_streams: exchanges }
    }

    fn collect(
        &mut self,
    ) -> impl Stream<Item = Result<(Vec<Self::BidOrder>, Vec<Self::AskOrder>), Box<dyn Error + Send + Sync>>> + '_ {
            let event_stream = stream! {
                let bid_heap = &mut BinaryHeap::<Reverse<Self::BidOrder>>::with_capacity(self.max_depth + 1);
                let ask_heap = &mut BinaryHeap::<Reverse<Self::AskOrder>>::with_capacity(self.max_depth + 1);
                loop {
                    match self.exchange_streams.next().await {
                        Some((exchange, event)) => {
                            match event {
                                Ok(snapshot) => {
                                    snapshot.bids.iter().for_each(|bid|{
                                        bid_heap.push(Reverse(BidLevel {
                                            price: bid[0],
                                            amount: bid[1],
                                            exchange,
                                        }));
                                        if bid_heap.len() > self.max_depth {
                                            bid_heap.pop();
                                        }
                                    });
    
                                    snapshot.asks.iter().for_each(|ask| {
                                        ask_heap.push(Reverse(AskLevel {
                                            price: ask[0],
                                            amount: ask[1],
                                            exchange,
                                        }));
                                        if ask_heap.len() > self.max_depth {
                                            ask_heap.pop();
                                        }
                                    });
                                    // Publish an event the moment an exchange publishes an updated orderbook
                                    yield Ok((
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
                                    ));
                                },
                                Err(e) => {
                                    yield Err(e)
                                }
                            }
    
                        },
                        None => {
                            yield Err(OrderbookError::StreamCancelled.into())
                        }
                    }
    
    
                }
            };
            event_stream
    }
}
