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

pub struct OrderbookSummaryService {}

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
            .with_max_depth(10)
            .with_symbol("ethbtc")
            .with_exchanges(&[ExchangeType::Binance, ExchangeType::Bitstamp]);

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
