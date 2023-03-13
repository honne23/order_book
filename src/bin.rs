use std::{env, error::Error};

use orderbook::server::{
    orderbook_rpc::orderbook_aggregator_server::OrderbookAggregatorServer, OrderbookSummaryService,
};
use tonic::transport::Server;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    let grpc_port = env::var("GRPC_PORT").unwrap();
    let addr = format!("[::0]:{grpc_port}").parse()?;
    let orderbook_server = OrderbookSummaryService {};
    Server::builder()
        .add_service(OrderbookAggregatorServer::new(orderbook_server))
        .serve(addr)
        .await?;
    Ok(())
}
