# warpforge-terminal

Sends log messages and progress updates over tcp streams.

## Simple Example

```rust
use warpforge_terminal::{logln, Logger};

Logger::set_global(Logger::new_local()).unwrap();

logln!("Hello, World!");
logln!("format {}", 42);
```

## More Examples

More examples can be found in the `examples` folder.
Run them using: `cargo run --example <name>`

The `client` example requires one of the `server_*` examples to be run first.
