
#[cfg(test)]
mod tests {
    use std::error::Error;

    use async_tungstenite::{tokio::connect_async, tungstenite::Message};
    use futures::{stream::StreamExt, SinkExt};
    
    #[tokio::test]
    async fn test_binance() -> Result<(), Box<dyn Error>> {
        let url = "wss://stream.binance.com:9443/ws/ethbtc@depth20@100ms";
        let (mut ws_stream, _) = connect_async(url).await?;
        let msg = ws_stream.next().await.ok_or("didn't receive anything")??;
        println!("{}", msg.to_string());
        Ok(())
    }

    #[tokio::test]
    async fn test_bitstamp() -> Result<(), Box<dyn Error>> {
        let url = "wss://ws.bitstamp.net";
        let (mut ws_stream, _) = connect_async(url).await?;
        ws_stream.send(Message::text(r#"{
            "event": "bts:subscribe",
        "data": {
            "channel": "order_book_btcusd"
        }}"#)).await?;

        // Confirm subscribed
        let msg = ws_stream.next().await.ok_or("didn't receive anything")??;
        println!("{}", msg.to_string());

        // Read first message
        let msg = ws_stream.next().await.ok_or("didn't receive anything")??;
        println!("{}", msg.to_string());
        Ok(())
        
    }

    
}

