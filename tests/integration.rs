#[cfg(test)]
mod tests {
    use std::{
        cmp::{Ordering, Reverse},
        error::Error,
        mem,
        num::ParseFloatError,
    };

    use async_tungstenite::{tokio::connect_async, tungstenite::Message};
    use futures::{stream::StreamExt, SinkExt};
    use std::collections::BinaryHeap;

    use serde::{de, Deserialize, Deserializer};

    #[derive(Debug, Default, Copy, Clone)]
    enum Exchange {
        #[default]
        Default,
        Binance,
        Bitstamp,
    }

    #[derive(Deserialize, Debug)]
    struct FeedSnapshot {
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

    // Asks: lowest is best
    // Bids: highest is best

    struct Orderbook {}

    #[derive(Debug, Default)]
    struct BidLevel {
        pub price: f64,
        pub amount: f64,
        pub exchange: Exchange,
    }

    impl Ord for BidLevel {
        fn cmp(&self, other: &Self) -> Ordering {
            (self.price / self.amount).total_cmp(&(other.price / other.amount))
        }
    }

    impl PartialOrd for BidLevel {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl PartialEq for BidLevel {
        fn eq(&self, other: &Self) -> bool {
            self.price / self.amount == other.price / other.amount
        }
    }

    impl Eq for BidLevel {}

    #[derive(Debug, Default)]
    struct AskLevel {
        pub price: f64,
        pub amount: f64,
        pub exchange: Exchange,
    }

    impl Ord for AskLevel {
        fn cmp(&self, other: &Self) -> Ordering {
            (self.amount / self.price).total_cmp(&(other.amount / other.price))
        }
    }

    impl PartialOrd for AskLevel {
        fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
            Some(self.cmp(other))
        }
    }

    impl PartialEq for AskLevel {
        fn eq(&self, other: &Self) -> bool {
            self.amount / self.price == other.amount / other.price
        }
    }

    impl Eq for AskLevel {}

    impl Orderbook {
        pub fn merge_feeds(
            feeds: Vec<(FeedSnapshot, Exchange)>,
            max_depth: usize,
        ) -> (Vec<BidLevel>, Vec<AskLevel>) {
            let mut bid_heap = BinaryHeap::<Reverse<BidLevel>>::with_capacity(max_depth + 1);
            let mut ask_heap = BinaryHeap::<Reverse<AskLevel>>::with_capacity(max_depth + 1);
            feeds.iter().for_each(|snapshot| {
                snapshot.0.bids.iter().for_each(|bid| {
                    bid_heap.push(Reverse(BidLevel {
                        price: bid[0],
                        amount: bid[1],
                        exchange: snapshot.1,
                    }));
                    if bid_heap.len() > max_depth {
                        bid_heap.pop();
                    }
                });
                snapshot.0.asks.iter().for_each(|ask| {
                    ask_heap.push(Reverse(AskLevel {
                        price: ask[0],
                        amount: ask[1],
                        exchange: snapshot.1,
                    }));
                    if ask_heap.len() > max_depth {
                        ask_heap.pop();
                    }
                })
            });

            (
                bid_heap
                    .into_sorted_vec()
                    .iter_mut()
                    .map(|x| mem::take(&mut x.0))
                    .collect::<Vec<BidLevel>>(),
                ask_heap
                    .into_sorted_vec()
                    .iter_mut()
                    .map(|x| mem::take(&mut x.0))
                    .collect::<Vec<AskLevel>>(),
            )
        }
    }

    #[derive(Deserialize, Debug)]
    struct BitstampSnapshot {
        pub data: FeedSnapshot,
    }

    #[tokio::test]
    async fn test_binance() -> Result<(), Box<dyn Error>> {
        let url = "wss://stream.binance.com:9443/ws/ethbtc@depth20@100ms";
        let (mut ws_stream, _) = connect_async(url).await?;
        let msg = ws_stream.next().await.ok_or("didn't receive anything")??;
        let data: FeedSnapshot = serde_json::from_str(msg.to_string().as_str())?;
        println!("{:?}", data.asks);
        Ok(())
    }

    #[tokio::test]
    async fn test_bitstamp() -> Result<(), Box<dyn Error>> {
        let url = "wss://ws.bitstamp.net";
        let (mut ws_stream, _) = connect_async(url).await?;
        ws_stream
            .send(Message::text(
                r#"{
                    "event": "bts:subscribe",
                    "data": {
                        "channel": "order_book_ethbtc"
                    }
                }"#,
            ))
            .await?;

        // Confirm subscribed
        let msg = ws_stream.next().await.ok_or("didn't receive anything")??;
        println!("{}", msg.to_string());
        // Read first message
        let msg = ws_stream.next().await.ok_or("didn't receive anything")??;
        let data: BitstampSnapshot = serde_json::from_str(msg.to_string().as_str())?;
        println!("{:?}", data.data.asks);
        Ok(())
    }
    #[tokio::test]
    async fn test_merged() -> Result<(), Box<dyn Error>> {
        let url = "wss://ws.bitstamp.net";
        let (mut ws_stream, _) = connect_async(url).await?;
        ws_stream
            .send(Message::text(
                r#"{
                    "event": "bts:subscribe",
                    "data": {
                        "channel": "order_book_ethbtc"
                    }
                }"#,
            ))
            .await?;

        // Confirm subscribed
        let msg = ws_stream.next().await.ok_or("didn't receive anything")??;

        // Read first message
        let msg = ws_stream.next().await.ok_or("didn't receive anything")??;
        let bit_data: BitstampSnapshot = serde_json::from_str(msg.to_string().as_str())?;

        let url = "wss://stream.binance.com:9443/ws/ethbtc@depth20@100ms";
        let (mut ws_stream, _) = connect_async(url).await?;
        let msg = ws_stream.next().await.ok_or("didn't receive anything")??;
        let binance_data: FeedSnapshot = serde_json::from_str(msg.to_string().as_str())?;
        let orderbook = Orderbook::merge_feeds(
            vec![
                (bit_data.data, Exchange::Bitstamp),
                (binance_data, Exchange::Binance),
            ],
            10,
        );

        println!("{:?}", orderbook.0);
        Ok(())
    }
}
