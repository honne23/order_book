# Crypto orderbook streamer

### High level architecture
This project has been built as an event-driven orderbook. Each exchange is modelled as a stream, these are then combined
into a merged stream when `Orderbook.collect()` is called. A new GRPC streaming request creates a new `SnapshotStream` that is
made available to the client; hence even if one exchange connection fails, the stream remains active.
```

                                                                         ┌───────────┐
                                                                         │ Binance   │
                                                                         │  Stream   │
                                                                         │           │
                                                                         │           │
                      ┌───────────────┐         ┌──────────────┐         └───┬───────┘
                      │               │         │              │             │
                      │  GRPC Stream  │         │    Merged    │◄────────────┘
 Clients  ◄───────────┤               │◄────────┤     Stream   │
                      │               │         │              │◄────────────┐
                      └───────────────┘         └──────────────┘             │
                                                                         ┌───┴───────┐
                                                                         │ Bitstamp  │
                                                                         │  Stream   │
                                                                         │           │
                                                                         │           │
                                                                         └───────────┘

```

### Ordering algorithm
Both bids and asks are ordered by the *best deal first*, facilitated by the `Ord` trait.
This means `AskLevel` is ordered by `amount` / `price` (greater = better deal) and `BidLevel` is 
ordered by `price` / `amount` (greater = better deal).

The algorithm tracks the asks and bids individually in a `HashSet`, and if the order has not already been recorded, it is
inserted into the appropriate *min-`BinaryHeap`*. If the number of elements in the binary heap exceeds the `max_depth`, the smallest
item is popped and removed from the heap, retaining only the best deals.

```rust
pub struct HashHeap<T> where T: Ord + Hash + Copy + Clone {
    heap: BinaryHeap<T>,
    tracker: HashSet<T>,
    capacity: usize
}
```

```rust
let mut ask_heap : HashHeap<Reverse<Self::AskOrder>> = HashHeap::with_capacity(self.max_depth);
let mut bid_heap : HashHeap<Reverse<Self::BidOrder>> = HashHeap::with_capacity(self.max_depth);
loop {
    match self.exchange_streams.next().await {
        Some((exchange, event)) => {
            match event {
                Ok(snapshot) => {
                    
                    snapshot.bids.iter().for_each(|bid|{
                        bid_heap.insert(Reverse(BidLevel {
                            price: bid[0],
                            amount: bid[1],
                            exchange,
                        }));
                    });

                    snapshot.asks.iter().for_each(|ask| {
                        ask_heap.insert(Reverse(AskLevel{
                            price: ask[0],
                            amount: ask[1],
                            exchange,
                        }));
                        
                    });
                    // Publish an event the moment an exchange publishes an updated orderbook
                    yield Ok((
                        bid_heap.into_sorted_vec()
                            .iter_mut()
                            .map(|x| mem::take(&mut x.0))
                            .collect::<Vec<BidLevel>>(),
                        ask_heap.into_sorted_vec()
                            .iter_mut()
                            .map(|x| mem::take(&mut x.0))
                            .collect::<Vec<AskLevel>>(),
                    ));
                },
                Err(e) => {
                    yield Err(e)
                }
            }

        },
        None => {
            yield Err(OrderbookError::StreamCancelled.into())
        }
    }
}
```


### Usage

Run `cargo install --path .` to install the binary and run the grpc server using the CLI

```
A program that merges the orderbooks from multiple exchanges, the CLI is used to configure rhe GRPC server :)

Usage: orderbook [OPTIONS] --max-depth <MAX_DEPTH> --port <PORT> --symbol <SYMBOL>

Options:
  -m, --max-depth <MAX_DEPTH>  Maximum depth of retrieved orders
  -e, --exchanges <EXCHANGES>  Exchanges to source orders from
  -p, --port <PORT>            Port to expose server
  -s, --symbol <SYMBOL>        Symbol to construct orderbook
  -h, --help                   Print help
  -V, --version                Print version
```

Example usage: 
```
./target/release/orderbook -p 50051 -e binance,bitstamp -m 10 -s ethbtc
```


### Further improvements

##### The `Exchange` trait
The `Exchange` trait could be refactored to decouple the network connection from the implementation in order to make testing easier,
likely as some sort of generic associative type. This way tests can be carried out without depending on a network connection.

##### The `HashHeap`
The HashHeap could potentially be refactored into a data structure that does not need to maintain multiple copies of the same `FeedSnapshot`
and instead works as follows:
1. Maintain the value of the `FeedSnapshot` in the binary heap.
2. Maintain a unique reference to the `FeedSnapshot` in the HashSet instead.
Achieving this could require some explicit lifetime definitions to guide the borrow checker, and ensure the references are threadsafe.


##### CI / CD
Adding CI / CD to carry out `cargo test` and `cargo clippy` would help to keep the repository healthy and avoid breaking changes being commited to master.


##### Use of `feature(return_position_impl_trait_in_trait)`
This experiemental feature had to be switched on in order to correctly define the `.collect()` function on the `Orderbook` (returning `impl Stream`). 
Otherwise the function would have to return a trait object instead, resulting in an additional heap allocation. This did not break compilation at this time
but should be duly noted. 

The alternative was to define an associated type on the `Orderbook` trait, however the experimental feature require to enable this (`impl-trait-existential-types`) resulted in an intellisense failure, with the language server marking the type as `Unknown`. Given these options, I felt it would be best to proced with the elected experimental feature in order to guide the programmer during development and avoid potentially undefined behaviour.