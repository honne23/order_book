use async_stream::stream;
use futures::Stream;
use std::{cmp::Reverse, collections::BinaryHeap, mem};
use tokio_stream::{StreamExt, StreamMap};

use crate::exchanges::{
   ExchangeType, SnapshotStream,
};

use self::levels::{AskLevel, BidLevel};
pub(crate) mod levels;
pub mod builder;




pub struct Orderbook {
    max_depth: usize,
    exchange_streams: StreamMap<ExchangeType, SnapshotStream>,
}



impl Orderbook {
    pub fn collect(&mut self) -> impl Stream<Item = (Vec<BidLevel>, Vec<AskLevel>)> + '_ {
        let event_stream = stream! {
            let bid_heap = &mut BinaryHeap::<Reverse<BidLevel>>::with_capacity(self.max_depth + 1);
            let ask_heap = &mut BinaryHeap::<Reverse<AskLevel>>::with_capacity(self.max_depth + 1); 
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
                // Publish an event the moment an exchange publishes an updated orderbook
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
        };
        event_stream
    }
}
