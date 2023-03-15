pub mod builder;
pub(crate) mod levels;
pub mod streaming_book;
mod hash_heap;

use std::error::Error;
use thiserror::Error;
use futures::Stream;
use tokio_stream::StreamMap;

use crate::exchanges::{ExchangeType, SnapshotStream};

#[derive(Error, Debug)]
enum OrderbookError {
    #[error("a stream has unexpectedly closed")]
    StreamCancelled,
}

pub trait Orderbook {
    /// A sortable type representing bids
    type BidOrder: Ord;
    /// A sortable type representing asks
    type AskOrder: Ord;

    /// Used to construct the orderbook within the orderbook builder.
    fn new(max_depth: usize, exchanges: StreamMap<ExchangeType, SnapshotStream>) -> Self;

    /// Collect a stream of orders, any errors from the stream are propagated up in the result.
    /// 
    /// Use `.pin_mut!()` to iterate over the stream.
    fn collect(
        &mut self,
    ) -> impl Stream<
        Item = Result<(Vec<Self::BidOrder>, Vec<Self::AskOrder>), Box<dyn Error + Send + Sync>>,
    > + '_;
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use futures::{pin_mut, StreamExt};

    use crate::{
        exchanges::ExchangeType,
        orderbook::{
            builder::{Empty, OrderbookBuilder},
            streaming_book::HeapedBook,
            Orderbook,
        },
    };

    use super::levels::{AskLevel, BidLevel};

    #[tokio::test]
    async fn test_orderbook() -> Result<(), Box<dyn Error>> {
        let orderbook_builder = OrderbookBuilder::<Empty>::new();
        let mut orderbook = orderbook_builder
            .with_max_depth(10)
            .with_symbol("ethbtc")
            .with_exchanges(&[ExchangeType::Binance, ExchangeType::Bitstamp])
            .build::<HeapedBook>()
            .await?;
        let orderbook_stream = orderbook.collect();
        pin_mut!(orderbook_stream); // needed for iteration

        while let Some(value) = orderbook_stream.next().await {
            let orders = value.unwrap();
            let spread = orders.1[0].price - orders.0[0].price;
            assert!(orders.0.len() == 10);
            assert!(orders.1.len() == 10);
            assert!(spread > 0.0);
            println!("Spread {spread} \n\n");
            break;
        }
        Ok(())
    }

    #[test]
    fn test_ask_ordering() {
        let mut asks = vec![
            AskLevel {
                price: 1.0,
                amount: 7.0,
                exchange: ExchangeType::Binance,
            },
            AskLevel {
                price: 3.0,
                amount: 12.0,
                exchange: ExchangeType::Binance,
            },
        ];
        asks.sort();
        assert!(asks[0] < asks[1]);
    }

    #[test]
    fn test_bids_ordering() {
        let mut bids = vec![
            BidLevel {
                price: 4.0,
                amount: 2.0,
                exchange: ExchangeType::Binance,
            },
            BidLevel {
                price: 1.4,
                amount: 8.4,
                exchange: ExchangeType::Binance,
            },
        ];
        bids.sort();
        assert!(bids[0] < bids[1]);
    }
}
