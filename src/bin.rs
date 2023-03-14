use std::str::FromStr;
use std::{env, error::Error};

use clap::Parser;
use orderbook::exchanges::ExchangeType;
use orderbook::server::{
    orderbook_rpc::orderbook_aggregator_server::OrderbookAggregatorServer, OrderbookSummaryService,
};
use orderbook::cli::{Args, CliError};
use tonic::transport::Server;



#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();
    let args = Args::parse();
    let addr = format!("[::0]:{}",args.port).parse()?;
    if !(args.max_depth > 0) {
        return Err(CliError::MaxDepthNotGreaterThanZeroError.into())
    }
    let mut exchanges : Vec<ExchangeType> = Vec::with_capacity(args.exchanges.len());
    for exchange in args.exchanges {
        exchanges.push(ExchangeType::from_str(&exchange)?);
    }
    let orderbook_server = OrderbookSummaryService::new(args.symbol, args.max_depth, &exchanges);
    Server::builder()
        .add_service(OrderbookAggregatorServer::new(orderbook_server))
        .serve(addr)
        .await?;
    Ok(())
}
