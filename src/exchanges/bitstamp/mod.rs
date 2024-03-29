use std::error::Error;

use async_trait::async_trait;
use async_tungstenite::{tokio::connect_async, tungstenite::Message};
use futures::{stream::StreamExt, SinkExt};

use serde::Deserialize;
use thiserror::Error;

use super::{Exchange, ExchangeType, FeedSnapshot, SecureWebsocketReceiver, SnapshotStream};

#[derive(Error, Debug)]
enum BitstampError {
    #[error("Bitstamp could not successfully connect to the websocket endpoint")]
    StreamFailed,
}

static EXCHANGE_URL: &str = "wss://ws.bitstamp.net";

#[derive(Deserialize, Debug)]
struct BitstampSnapshot {
    pub data: FeedSnapshot,
}

#[derive(Deserialize, Debug)]
struct BitstampConnectResponse {
    event: String
}

pub struct Bitstamp {}

#[async_trait]
impl Exchange for Bitstamp {
    async fn connect(&self, symbol: String, max_depth: usize) -> Result<SnapshotStream, Box<dyn Error>> {
        let (mut ws_stream, _) = connect_async(EXCHANGE_URL).await?;
        ws_stream
            .send(Message::text(format!(
                "{{
                    \"event\": \"bts:subscribe\",
                    \"data\": {{
                        \"channel\": \"order_book_{symbol}\"
                    }}
                }}"
            )))
            .await?;

        // Confirm subscribed
        let msg = futures::StreamExt::next(&mut ws_stream)
            .await
            .ok_or("didn't receive anything")??;
        let data: BitstampConnectResponse = serde_json::from_str(msg.to_string().as_str())?;
        if data.event.as_str() != "bts:subscription_succeeded" {
            return Err(BitstampError::StreamFailed.into())
        }
        // Check the msg here
        let (_, mut ws_receiver) = ws_stream.split();
        
        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<
            Result<FeedSnapshot, Box<dyn Error + Sync + Send>>,
        >();
        let rx = Box::pin(async_stream::stream! {
              while let Some(item) = rx.recv().await {
                  yield item;
              }
        }) as SnapshotStream;
        tokio::spawn(async move {
            loop {
                // Async closure for more egonomic error handling
                let channel_result = async move |ws: &mut SecureWebsocketReceiver| -> Result<
                    FeedSnapshot,
                    Box<dyn Error + Send + Sync>,
                > {
                    let msg = futures::StreamExt::next(ws)
                        .await
                        .ok_or("didn't receive anything")??;
                    let data: BitstampSnapshot = serde_json::from_str(msg.to_string().as_str())?;
                    Ok(data.data)
                }(&mut ws_receiver)
                .await;

                match channel_result {
                    Err(e) => tx.send(Err(e)).unwrap(),
                    Ok(snapshot) => tx.send(Ok(snapshot)).unwrap(),
                };
            }
        });

        Ok(rx)
    }

    fn name(&self) -> ExchangeType {
        ExchangeType::Bitstamp
    }
}
