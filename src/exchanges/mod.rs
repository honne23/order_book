use std::{error::Error, num::ParseFloatError, pin::Pin};
use std::hash::Hash;

use async_trait::async_trait;
use serde::{de, Deserialize, Deserializer};
use tokio_stream::Stream;

pub mod binance;
pub mod bitstamp;

#[derive(Debug, Default, Copy, Clone, Eq, PartialEq, Hash)]
pub enum ExchangeType {
    #[default]
    Default,
    Binance,
    Bitstamp,
}

impl ToString for ExchangeType {
    fn to_string(&self) -> String {
        match *self {
            Self::Binance => String::from("Binance"),
            Self::Bitstamp => String::from("Bitstamp"),
            Self::Default => String::from("Default")
        }
    }
}

pub(crate) type SnapshotStream =
    Pin<Box<dyn Stream<Item = Result<FeedSnapshot, Box<dyn Error + Send + Sync>>> + Send>>;

#[async_trait]
pub(crate) trait Exchange {
    async fn connect(&self, symbol: String) -> Result<SnapshotStream, Box<dyn Error>>;

    fn name(&self) -> ExchangeType;
}

#[derive(Deserialize, Debug, Default)]
pub(crate) struct FeedSnapshot {
    #[serde(deserialize_with = "from_str_floats")]
    pub bids: Vec<[f64; 2]>,
    #[serde(deserialize_with = "from_str_floats")]
    pub asks: Vec<[f64; 2]>,
}

fn from_str_floats<'de, D>(deserializer: D) -> Result<Vec<[f64; 2]>, D::Error>
where
    D: Deserializer<'de>,
{
    let unparsed_prices: Vec<[String; 2]> = Deserialize::deserialize(deserializer)?;
    let parsed_prices = parse_prices(unparsed_prices).map_err(de::Error::custom)?;
    Ok(parsed_prices)
}

fn parse_prices(data: Vec<[String; 2]>) -> Result<Vec<[f64; 2]>, ParseFloatError> {
    let mut parsed_deals: Vec<[f64; 2]> = Vec::with_capacity(data.len());
    for level in data {
        parsed_deals.push([level[0].parse::<f64>()?, level[1].parse::<f64>()?]);
    }
    Ok(parsed_deals)
}
