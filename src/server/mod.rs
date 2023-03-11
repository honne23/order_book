pub mod orderbook_rpc {
    tonic::include_proto!("orderbook"); // The string specified here must match the proto package name
}

use futures::{pin_mut, StreamExt};
use orderbook_rpc::orderbook_aggregator_server::OrderbookAggregator;
use orderbook_rpc::{Empty, Level, Summary};
use tokio::sync::mpsc;
use tonic::{Request, Response, Status};
use tokio_stream::wrappers::ReceiverStream;

use crate::exchanges::ExchangeType;
use crate::orderbook::builder::{OrderbookBuilder, Empty as EmptyOrderbook};
use crate::orderbook::levels::{BidLevel, AskLevel};

pub struct OrderbookServer {}

#[tonic::async_trait]
impl OrderbookAggregator for OrderbookServer {
    type BookSummaryStream = ReceiverStream<Result<Summary, Status>>;

    async fn book_summary(&self, _request: Request<Empty>) -> Result<Response<Self::BookSummaryStream>, Status> {
        let (mut tx, rx) = mpsc::channel::<Result<Summary, Status>>(4);
        tokio::spawn(async move {
            let orderbook_builder = OrderbookBuilder::<EmptyOrderbook>::new();
            let mut orderbook = orderbook_builder
                .with_max_depth(10)
                .with_symbol("ethbtc")
                .with_exchange(ExchangeType::Binance)
                .with_exchange(ExchangeType::Bitstamp)
                .build().await.unwrap();

            let orderbook_stream = orderbook.collect();
            pin_mut!(orderbook_stream); // needed for iteration

            while let Some(value) = orderbook_stream.next().await {
                let summary_bids = value.0.iter().map(|bid|Level{
                    price: bid.price,
                    amount: bid.amount,
                    exchange: bid.exchange.to_string()
                }).rev().collect::<Vec<Level>>();

                let summary_asks = value.1.iter().map(|ask|Level{
                    price: ask.price,
                    amount: ask.amount,
                    exchange: ask.exchange.to_string()
                }).rev().collect::<Vec<Level>>();

                tx.send(Ok(Summary{
                    spread: summary_bids[0].price - summary_asks[0].price,
                    bids: summary_bids,
                    asks: summary_asks
                })).await.unwrap();
            }
        });

        Ok(Response::new(ReceiverStream::new(rx)))
    }
   
}