pub mod builder;
pub mod streaming_book;
pub(crate) mod levels;

use std::error::Error;

use futures::Stream;
use tokio_stream::StreamMap;

use crate::exchanges::{ExchangeType, SnapshotStream};

pub trait Orderbook {
    type BidOrder: Ord;
    type AskOrder: Ord;

    fn new(max_depth: usize, exchanges: StreamMap<ExchangeType, SnapshotStream>) -> Self;

    fn collect(
        &mut self,
    ) -> impl Stream<Item = Result<(Vec<Self::BidOrder>, Vec<Self::AskOrder>), Box<dyn Error + Send + Sync>>> + '_;
}

#[cfg(test)]
mod tests {
    use std::error::Error;

    use futures::{pin_mut, StreamExt};

    use crate::{orderbook::{builder::{OrderbookBuilder, Empty}, streaming_book::HeapedBook, Orderbook}, exchanges::ExchangeType};

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
            assert!(value.is_ok());
            let orders = match value {
                Ok(orders) => orders,
                Err(e) => {
                    println!("ERROR {:?}", e);
                    continue;
                }
            };
            let spread = orders.1[orders.1.len()-1].price - orders.0[orders.0.len()-1].price;
            assert!(orders.0.len() == 10);
            assert!(orders.1.len() == 10);
            assert!(spread > 0.0);
            println!("Spread {spread} \n\n");
        }
        Ok(())
    }
}
