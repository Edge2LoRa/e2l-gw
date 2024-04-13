# Edge2LoRa Gateway Edge Module

The Edge2LoRa Gateway Edge Module is the module that enables the Edge Computing in LoRaWAN Gateways. It manages:

- Group Key Agreement to compute the new session edge keys
- Receive/Send Commands to the [E2L Distributed Module (DM)](https://github.com/Edge2LoRa/e2l-distributed-module)
- Aggregates and sends data to the DM
## Prerequisites

### Rust

To run the Edge2LoRa Gateway Edge Module you need to install Rust and its compiler in your system. Please, follow the instruction in the [official website](https://www.rust-lang.org/tools/install).

### Proto Buffer

The E2L GW Edge Module make use of the [tonic crate](https://github.com/hyperium/tonic), which offers a Rust implementation for [gRPC](https://grpc.io/).
gRPC makes use of [Protocol Buffers](https://protobuf.dev/), that need to be installed in your system. Follow this [link](https://grpc.io/docs/protoc-installation/) for the installation instruction.

## Build & Run

To build:

```bash
cargo build
```

To build and run:
```bash
cargo run
```
