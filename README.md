# Crypto orderbook streamer

### High level architecture
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
