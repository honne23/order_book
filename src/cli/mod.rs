use clap::{Parser, command};
use thiserror::Error;



#[derive(Error, Debug)]
pub enum CliError {
    #[error("max depth must be greater than zero")]
    MaxDepthNotGreaterThanZeroError,
}



/// A program that merges the orderbooks from multiple exchanges, the CLI is used to configure the GRPC server :)
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    /// Maximum depth of retrieved orders
    #[arg(short, long)]
    pub max_depth: usize,

    /// Exchanges to source orders from
    #[arg(short, long, value_delimiter = ',')]
    pub exchanges: Vec<String>,

    /// Port to expose server
    #[arg(short, long)]
    pub port: String,

    /// Symbol to construct orderbook
    #[arg(short, long)]
    pub symbol: String
}



