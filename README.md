
# KSU Capstone Project

## Running in broswer

First install `wasm-pack`

```sh
cargo install wasm-pack
```

Then builds can be done like so:

```sh
cargo build
cargo run -p server
```

> [!WARNING]
> The server build.rs does the wasm-pack build of `client`, but not the native build.
> There may be analyzer errors due to a missing native build.
