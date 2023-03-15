use std::error::Error;

use async_trait::async_trait;
use async_tungstenite::tokio::connect_async;
use futures::stream::StreamExt;

use super::{Exchange, ExchangeType, FeedSnapshot, SecureWebsocketReceiver, SnapshotStream};

static EXCHANGE_URL: &str = "wss://stream.binance.com:9443/ws/";

pub(crate) struct Binance {}

#[async_trait]
impl Exchange for Binance {
    async fn connect(&self, symbol: String, max_depth: usize) -> Result<SnapshotStream, Box<dyn Error>> {
        let (ws_stream, _) = connect_async(format!("{EXCHANGE_URL}{symbol}@depth{max_depth}@100ms")).await?;

        let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<
            Result<FeedSnapshot, Box<dyn Error + Sync + Send>>,
        >();
        let rx = Box::pin(async_stream::stream! {
              while let Some(item) = rx.recv().await {
                  yield item;
              }
        }) as SnapshotStream;
        let (_, mut ws_receiver) = ws_stream.split();
        tokio::spawn(async move {
            loop {
                let channel_result = async move |ws: &mut SecureWebsocketReceiver| -> Result<
                    FeedSnapshot,
                    Box<dyn Error + Send + Sync>,
                > {
                    let msg = ws.next().await.ok_or("didn't receive anything")??;
                    let binance_data: FeedSnapshot =
                        serde_json::from_str(msg.to_string().as_str())?;
                    Ok(binance_data)
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
        ExchangeType::Binance
    }
}
