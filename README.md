# Simple node using Routing library

Kadmelia-like routing node written in Rust

## Build
`cargo build`

## Run
1. `cd target/debug/`
2. For the first node run `RUST_LOG=debug,info ./routing-simple-node -f`
3. For the rest of nodes just run `RUST_LOG=debug,info ./routing-simple-node`

**Note:** in order for nodes to bootstrap correctly in LAN
you may need to place both files from configs/ folder next to executable file.
It can be done by using `cp ../../configs/* .`

## Routing library
This project is powered by MaidSafe Routing library https://github.com/maidsafe/routing
