# source2-demo

[![Crates.io](https://img.shields.io/crates/v/source2-demo)](https://crates.io/crates/source2-demo)
[![Documentation](https://img.shields.io/docsrs/source2-demo)](https://docs.rs/source2-demo)
[![License](https://img.shields.io/crates/l/source2-demo)](https://github.com/Rupas1k/source2-demo)

`source2-demo` is a Rust library for parsing Source 2 engine demo files.

## Supported Games

- Dota 2
- Deadlock
- Counter-Strike 2

## Installation

Add the following to your `Cargo.toml` and enable the feature for the game you want to parse:

```toml
[dependencies]
# For Dota 2
source2-demo = { version = "0.4", features = ["dota"] }

# For Deadlock
# source2-demo = { version = "0.4", features = ["deadlock"] }

# For Counter-Strike 2
# source2-demo = { version = "0.4", features = ["cs2"] }
```

## Quick Start: Parsing Chat Messages

Here's a simple program that prints chat messages from a Dota 2 replay. It handles the `CDotaUserMsgChatMessage` protobuf message and prints the player's name and their message.

More examples can be found in the `d2-examples` and `dl-examples` directories.

```rust ignore
use source2_demo::prelude::*;
use source2_demo::proto::*;

// Create a struct that implements the Default trait
#[derive(Default)]
struct Chat;

// Mark the impl block with the observer attribute
#[observer(all)]
impl Chat {
    // Use the on_message attribute to mark the protobuf message handler
    #[on_message]
    fn handle_chat_msg(
        &mut self,
        ctx: &Context,
        chat_msg: CDotaUserMsgChatMessage, // Use any protobuf message as an argument
    ) -> ObserverResult {
        if let Ok(pr) = ctx.entities().get_by_class_name("CDOTA_PlayerResource") {
            let name: String = property!(
                pr,
                "m_vecPlayerData.{:04}.m_iszPlayerName",
                chat_msg.source_player_id()
            );
            println!("{}: {}", name, chat_msg.message_text());
        }
        Ok(())
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create a parser
    let mut parser = Parser::from_reader(std::fs::File::open("replay.dem")?)?;

    // Register observers
    parser.register_observer::<Chat>();

    // Run the parser
    parser.run_to_end()?;

    Ok(())
}
```

## Building Examples

To build the examples, clone the repository and use Cargo:

```shell
git clone https://github.com/Rupas1k/source2-demo
cd source2-demo

# Build examples for a specific game
cd dl-examples # d2-examples
cargo build --release
```

## Features

The crate supports the following cargo features:

- **`dota`** - Enable Dota 2 replay parsing (includes Dota 2 protobufs)
- **`deadlock`** - Enable Deadlock replay parsing (includes Citadel protobufs)
- **`cs2`** - Enable Counter-Strike 2 replay parsing (includes CS2 protobufs)
- **`unsafe`** - Disable bounds checking in the reader for improved performance.
- **`mimalloc`** (default) - Use mimalloc as the global allocator for improved performance

You can enable multiple game features if needed:

```toml
source2-demo = { version = "*", features = ["dota", "cs2"] }
```

You can disable mimalloc if it causes issues on your platform (e.g., WebAssembly):

```toml
source2-demo = { version = "*", default-features = false }
```

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Credits

This project was inspired by and builds upon the work of other Source 2 demo parsers:

- [clarity](https://github.com/skadistats/clarity) - Java-based Source 2 replay parser
- [manta](https://github.com/dotabuff/manta) - Go-based Dota 2 replay parser


## License

This project is dual-licensed under either:

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.

