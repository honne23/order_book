use std::{error::Error, mem};

use async_trait::async_trait;
use async_tungstenite::{
    tokio::connect_async,
    tungstenite::Message,
};
use futures::{
    stream::StreamExt,
    SinkExt,
};

use serde::Deserialize;

use super::{Exchange, FeedSnapshot, ExchangeType, SnapshotStream};

static EXCHANGE_URL: &str = "wss://ws.bitstamp.net";



#[derive(Deserialize, Debug)]
struct BitstampSnapshot {
    pub data: FeedSnapshot,
}
pub struct Bitstamp {}

#[async_trait]
impl Exchange for Bitstamp {
    async fn connect(
        &self, symbol: String
    ) -> Result<SnapshotStream, Box<dyn Error>>
    {
        let (mut ws_stream, _) = connect_async(EXCHANGE_URL).await?;
        ws_stream
            .send(Message::text(
                format!("{{
                    \"event\": \"bts:subscribe\",
                    \"data\": {{
                        \"channel\": \"order_book_{symbol}\"
                    }}
                }}"),
            ))
            .await?;

        // Confirm subscribed
        let msg = futures::StreamExt::next(&mut ws_stream).await.ok_or("didn't receive anything")??;
        // Check the msg here
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
                let msg = futures::StreamExt::next(&mut ws_receiver)
                    .await
                    .ok_or("didn't receive anything")
                    .unwrap()
                    .unwrap();
                let mut data: BitstampSnapshot =
                    serde_json::from_str(msg.to_string().as_str()).unwrap();
                tx.send(Ok(mem::take(&mut data.data))).await.unwrap();
            }
        });

        Ok(rx)
    }

    fn name(&self) -> ExchangeType {
        ExchangeType::Bitstamp
    }
}
