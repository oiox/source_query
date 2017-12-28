## source_query
[![](https://img.shields.io/crates/v/source_query.svg?style=flat-square)](https://crates.io/crates/source_query) [![](https://img.shields.io/badge/doc-docs.rs-blue.svg?style=flat-square)](https://docs.rs/source_query) ![](https://img.shields.io/github/license/oiox/source_query.svg?style=flat-square)

A Rust crate for querying Source game servers with a simple, blocking API.

### Examples

```rust
use source_query::info;

let resp = info::query("52.61.24.34:27015", None)?;

println!("[{players}/{max_players}] {name}",
         players     = resp.players,
         max_players = resp.max_players,
         name        = resp.name);
```
