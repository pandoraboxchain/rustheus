# Prometheus node repository

Prometheus node written in Rust

## Build
`cargo build`

## Run
1. `cd target/debug/`
2. For the first node run `RUST_LOG=debug,info ./rustheus -f`
3. For the rest of nodes just run `RUST_LOG=debug,info ./rustheus -n N` where N is unique node number with its own storage
4. To issue commands you can use `telnet localhost 1234` or better `rlwrap telnet localhost 1234` for command history support

**Note:** in order for nodes to bootstrap correctly in LAN
you may need to place both files from configs/ folder next to executable file.
It can be done by using `cp ../../configs/* .`

## Development
This repository contains configs to build and debug project from Visual Studio Code. LLDB Debugger plugin is required for debug. Rust (rls) package is recommended for faster compile-and-run cycle

## Credits
P2P communication layer is powered by MaidSafe Routing library https://github.com/maidsafe/routing  
Contains parts from Parity Bitcoin implementation https://github.com/paritytech/parity-bitcoin
