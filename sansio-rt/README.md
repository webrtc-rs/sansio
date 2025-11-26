# sansio-rt

Runtime abstraction for the `sansio` ecosystem, providing a unified interface for both `smol` and `tokio` async runtimes.

## Features

- **Runtime Flexibility**: Choose between smol or tokio at compile time
- **CPU Pinning**: Pin executor threads to specific CPU cores
- **Thread Naming**: Name executor threads for debugging
- **Consistent API**: Uniform API across both runtimes

## Quick Start

### With smol (default)

```toml
[dependencies]
sansio-rt = "0.0.5"
```

```rust
use sansio_rt::LocalExecutorBuilder;

fn main() {
    LocalExecutorBuilder::default()
        .run(async {
            println!("Running on smol!");
        });
}
```

### With tokio

```toml
[dependencies]
sansio-rt = { version = "0.0.5", default-features = false, features = ["runtime-tokio"] }
```

```rust
use sansio_rt::LocalExecutorBuilder;

fn main() {
    LocalExecutorBuilder::default()
        .run(async {
            println!("Running on tokio!");
        });
}
```

## Documentation

For detailed documentation, see:
- [API Documentation](https://docs.rs/sansio-rt)
- [Local Executor Guide](doc/LocalExecutor.md)
- [Main sansio crate](https://docs.rs/sansio)

## Feature Flags

- `runtime-smol` (default): Use smol's LocalExecutor
- `runtime-tokio`: Use tokio's LocalSet

## License

Licensed under either of:

- Apache License, Version 2.0 ([LICENSE-APACHE](../LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
- MIT license ([LICENSE-MIT](../LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.
