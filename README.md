# Hack.chat Library For Rust

[![Crates.io](https://img.shields.io/crates/v/hackchat.svg)](https://crates.io/crates/hackchat) | [Documentation](https://gkbrk.github.io/rust-hackchat/hackchat/index.html)

A client library for Hack.chat.

This library allows you to make custom clients and bots for Hack.chat using Rust.

Here's an example that connects to a channel and prints the chat logs to the console.

```rust
extern crate hackchat;
use hackchat::{ChatClient, ChatEvent};

fn main() {
    let mut conn = ChatClient::new("TestBot", "botDev"); //Connects to the ?botDev channel
    conn.start_ping_thread(); //Sends ping packets regularly

    for event in conn.iter() {
        match event {
            ChatEvent::Message(nick, message, trip_code) => {
                println!("<{}> {}", nick, message);
            },
            ChatEvent::JoinRoom(nick) => {
                println!("{} joined the room", nick);
            },
            _ => {}
        }
    }
}
```

Here's some example output from this code:

```
gkbrk joined the room
<gkbrk> Hey there!
<gkbrk> Testing... 1, 2, 3
```
