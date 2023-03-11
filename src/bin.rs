use std::{error::Error, env};

use orderbook::server::{OrderbookSummaryService, orderbook_rpc::orderbook_aggregator_server::OrderbookAggregatorServer};
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>>  {
    let grpc_port = env::var("GRPC_PORT").unwrap();
    let addr = format!("[::0]:{}", grpc_port).parse()?;
    let orderbook_server = OrderbookSummaryService{};
    Server::builder()
        .add_service(OrderbookAggregatorServer::new(orderbook_server))
        .serve(addr)
        .await?;
    Ok(())
}