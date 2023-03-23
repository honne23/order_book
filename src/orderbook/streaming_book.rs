use async_stream::stream;
use futures::Stream;
use std::{cmp::Reverse, error::Error, mem};
use tokio_stream::{StreamExt, StreamMap};

use crate::{exchanges::{ExchangeType, SnapshotStream}, orderbook::{OrderbookError, hash_heap::HashHeap}};

use super::{
    levels::{AskLevel, BidLevel},
    Orderbook,
};



pub struct HeapedBook {
    max_depth: usize,
    exchange_streams: StreamMap<ExchangeType, SnapshotStream>,
}

impl Orderbook for HeapedBook {
    type AskOrder = AskLevel;
    type BidOrder = BidLevel;

    fn new(max_depth: usize, exchanges: StreamMap<ExchangeType, SnapshotStream>) -> Self {
        Self {
            max_depth,
            exchange_streams: exchanges,
        }
    }

    fn collect(
        &mut self,
    ) -> impl Stream<
        Item = Result<(Vec<Self::BidOrder>, Vec<Self::AskOrder>), Box<dyn Error + Send + Sync>>,
    > + '_ {
        stream! {
            let mut ask_heap : HashHeap<Reverse<Self::AskOrder>> = HashHeap::with_capacity(self.max_depth);
            let mut bid_heap : HashHeap<Reverse<Self::BidOrder>> = HashHeap::with_capacity(self.max_depth);
            loop {
                match self.exchange_streams.next().await {
                    Some((exchange, event)) => {
                        match event {
                            Ok(snapshot) => {
                                
                                snapshot.bids.iter().for_each(|bid|{
                                    bid_heap.insert(Reverse(BidLevel {
                                        price: bid[0],
                                        amount: bid[1],
                                        exchange,
                                    }));
                                });

                                snapshot.asks.iter().for_each(|ask| {
                                    ask_heap.insert(Reverse(AskLevel{
                                        price: ask[0],
                                        amount: ask[1],
                                        exchange,
                                    }));
                                    
                                });
                                // Publish an event the moment an exchange publishes an updated orderbook
                                yield Ok((
                                    bid_heap.into_sorted_vec()
                                        .iter_mut()
                                        .map(|x| mem::take(&mut x.0))
                                        .collect::<Vec<BidLevel>>(),
                                    ask_heap.into_sorted_vec()
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
        }
    }
}
