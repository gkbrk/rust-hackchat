extern crate hackchat;

use hackchat::{ChatClient, ChatEvent};

fn main() {
    let mut conn = ChatClient::new("RustBot", "botDev");
    conn.start_ping_thread();

    for event in conn.iter() {
        match event {
            ChatEvent::Message(nick, message, trip_code) => {
                println!("<{}> {}", nick, message);
            },
            _ => {}
        }
    }
}
