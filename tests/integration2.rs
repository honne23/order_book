
#[cfg(test)]
mod test_lib {
    use std::error::Error;

    use futures::{pin_mut, StreamExt};
    use orderbook::{orderbook::builder::{OrderbookBuilder, Empty}, exchanges::ExchangeType};


    #[tokio::test]
    async fn test_orderbook() -> Result<(), Box<dyn Error>> {
        let orderbook_builder = OrderbookBuilder::<Empty>::new();
        let mut orderbook = orderbook_builder
            .with_max_depth(10)
            .with_symbol("ethbtc")
            .with_exchange(ExchangeType::Binance)
            .with_exchange(ExchangeType::Bitstamp)
            .build().await?;
        let orderbook_stream = orderbook.collect();
        pin_mut!(orderbook_stream); // needed for iteration

        while let Some(value) = orderbook_stream.next().await {
            println!("got {:?} \n\n", value);
        }
        Ok(())
        
    }

}