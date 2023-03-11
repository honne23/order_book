use std::error::Error;

use async_trait::async_trait;
use async_tungstenite::tokio::connect_async;
use futures::stream::StreamExt;

use super::{Exchange, ExchangeType, FeedSnapshot, SnapshotStream};

static EXCHANGE_URL: &str = "wss://stream.binance.com:9443/ws/";

pub(crate) struct Binance {}

#[async_trait]
impl Exchange for Binance {
    async fn connect(&self, symbol: String) -> Result<SnapshotStream, Box<dyn Error>> {
        let (ws_stream, _) = connect_async(format!("{EXCHANGE_URL}{symbol}@depth20@100ms")).await?;
        let (_, mut ws_receiver) = ws_stream.split();
        let (tx, mut rx) =
            tokio::sync::mpsc::channel::<Result<FeedSnapshot, Box<dyn Error + Sync + Send>>>(1);
        let rx = Box::pin(async_stream::stream! {
              while let Some(item) = rx.recv().await {
                  yield item;
              }
        }) as SnapshotStream;
        tokio::spawn(async move {
            loop {
                let msg = ws_receiver
                    .next()
                    .await
                    .ok_or("didn't receive anything")
                    .unwrap()
                    .unwrap();
                let binance_data: FeedSnapshot =
                    serde_json::from_str(msg.to_string().as_str()).unwrap();
                tx.send(Ok(binance_data)).await.unwrap();
            }
        });

        Ok(rx)
    }

    fn name(&self) -> ExchangeType {
        ExchangeType::Binance
    }
}
