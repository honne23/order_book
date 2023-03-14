pub mod orderbook_rpc {
    tonic::include_proto!("orderbook");
}

use futures::{pin_mut, StreamExt};
use orderbook_rpc::orderbook_aggregator_server::OrderbookAggregator;
use orderbook_rpc::{Empty, Level, Summary};
use tokio::sync::mpsc;
use tokio_stream::wrappers::UnboundedReceiverStream;
use tonic::{Request, Response, Status};

use crate::exchanges::ExchangeType;
use crate::orderbook::{
    builder::{Empty as EmptyOrderbook, OrderbookBuilder},
    streaming_book::HeapedBook,
    Orderbook,
};

pub struct OrderbookSummaryService {
    max_depth: usize,
    symbol: String,
    exchanges: Vec<ExchangeType>
}

impl OrderbookSummaryService {
    pub fn new<S: Into<String>>(symbol: S, max_depth:usize, exchanges: &[ExchangeType]) -> Self {
        Self { 
            symbol: symbol.into(),
            max_depth,
            exchanges: exchanges.into()
         }
    }
}

#[tonic::async_trait]
impl OrderbookAggregator for OrderbookSummaryService {
    type BookSummaryStream = UnboundedReceiverStream<Result<Summary, Status>>;

    async fn book_summary(
        &self,
        _request: Request<Empty>,
    ) -> Result<Response<Self::BookSummaryStream>, Status> {
        let (tx, rx) = mpsc::unbounded_channel::<Result<Summary, Status>>();
        let orderbook_builder = OrderbookBuilder::<EmptyOrderbook>::new();
        let orderbook_builder = orderbook_builder
            .with_max_depth(self.max_depth)
            .with_symbol(self.symbol.clone())
            .with_exchanges(&self.exchanges);

        let mut orderbook = match orderbook_builder.build::<HeapedBook>().await {
            Ok(orderbook) => orderbook,
            Err(e) => {
                error!("orderbook could not be created: {e}");
                return Err(Status::aborted("could not create orderbook"));
            }
        };
        info!("new orderbook initialised");
        tokio::spawn(async move {
            let orderbook_stream = orderbook.collect();
            pin_mut!(orderbook_stream);
            while let Some(event) = orderbook_stream.next().await {
                match event {
                    Ok(snapshot) => {
                        let summary_bids = snapshot
                            .0
                            .iter()
                            .map(|bid| Level {
                                price: bid.price,
                                amount: bid.amount,
                                exchange: bid.exchange.to_string(),
                            })
                            .collect::<Vec<Level>>();

                        let summary_asks = snapshot
                            .1
                            .iter()
                            .map(|ask| Level {
                                price: ask.price,
                                amount: ask.amount,
                                exchange: ask.exchange.to_string(),
                            })
                            .collect::<Vec<Level>>();

                        tx.send(Ok(Summary {
                            spread: summary_asks[0].price - summary_bids[0].price,
                            bids: summary_bids,
                            asks: summary_asks,
                        }))
                        .unwrap();
                    }
                    Err(e) => {
                        error!("stream returned none: {e}");
                        tx.send(Err(Status::data_loss(
                            "could not retrieve update from orderbook",
                        )))
                        .unwrap();
                    }
                };
            }
        });

        Ok(Response::new(UnboundedReceiverStream::new(rx)))
    }
}

#[cfg(test)]
mod tests {
    use std::{error::Error, thread, time};

    use futures::StreamExt;
    use tonic::transport::Server;
    use crate::{exchanges::ExchangeType, server::{OrderbookSummaryService, orderbook_rpc::{orderbook_aggregator_server::OrderbookAggregatorServer, orderbook_aggregator_client::OrderbookAggregatorClient, Empty}}};
  

    #[tokio::test]
    async fn test_server() -> Result<(), Box<dyn Error>> {
        tokio::spawn(async move {
            let addr = format!("[::0]:{}",8080).parse().unwrap();
            let orderbook_server = OrderbookSummaryService::new("ethbtc", 10, &vec![ExchangeType::Binance, ExchangeType::Bitstamp]);
           
            // Run server
            Server::builder()
                .add_service(OrderbookAggregatorServer::new(orderbook_server))
                .serve(addr)
                .await.unwrap();
        });
        thread::sleep(time::Duration::from_secs(1u64));
        let mut client = OrderbookAggregatorClient::connect(format!("http://[::0]:{}",8080)).await?;
        let req = tonic::Request::new(Empty{});
        let mut stream = client.book_summary(req).await?.into_inner();
        while let Some(item) = stream.next().await {
            let summary = item?;
            assert!(summary.bids.len() == 10);
            assert!(summary.asks.len()== 10);
            assert!(summary.spread > 0.0);
            println!("Spread {} \n\n", summary.spread);
            break
        }

        Ok(())
        
    }
}
