
#[cfg(test)]
mod test_lib {
    use std::error::Error;

    use futures::{pin_mut, StreamExt};
    use orderbook::{orderbook::OrderbookBuilder, exchanges::ExchangeType};


    #[tokio::test]
    async fn test_orderbook() -> Result<(), Box<dyn Error>> {
        let mut orderbook_builder = OrderbookBuilder::new();
        let mut orderbook = orderbook_builder
            .with_max_depth(10)
            .with_exchange(ExchangeType::Binance)
            .with_exchange(ExchangeType::Bitstamp)
            .with_symbol("ethbtc")
            .build().await?;
        let orderbook_stream = orderbook.collect().await;
        pin_mut!(orderbook_stream); // needed for iteration

        while let Some(value) = orderbook_stream.next().await {
            println!("got {:?} \n\n", value);
        }
        Ok(())
        
    }

}