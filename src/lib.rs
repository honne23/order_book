#![feature(async_closure)]
#![feature(return_position_impl_trait_in_trait)]
pub mod exchanges;
pub mod orderbook;
pub mod server;
pub mod cli;

#[macro_use]
extern crate log;
extern crate pretty_env_logger;
