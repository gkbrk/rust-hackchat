//! #rust-hackchat
//! A client library for Hack.chat.
//!
//! This library allows you to make custom clients and bots for Hack.chat using Rust.
//!
//! #Examples
//!
//! ```
//! extern crate hackchat;
//! use hackchat::{ChatClient, ChatEvent};
//!
//! fn main() {
//!     let mut conn = ChatClient::new("TestBot", "botDev"); //Connects to the ?botDev channel
//!     conn.start_ping_thread(); //Sends ping packets regularly
//!
//!     for event in conn.iter() {
//!         match event {
//!             ChatEvent::Message(nick, message, trip_code) => {
//!                 println!("<{}> {}", nick, message);
//!             },
//!             _ => {}
//!         }
//!     }
//! }
//! ```

extern crate websocket;
#[macro_use] extern crate serde_json;
extern crate rustc_serialize;

use std::thread;

use rustc_serialize::json;

use websocket::{Client, Message, WebSocketStream};
use websocket::message::Type;
use websocket::client::request::Url;

use websocket::sender::Sender;
use websocket::receiver::Receiver;

use websocket::ws::sender::Sender as SenderTrait;
use websocket::ws::receiver::Receiver as ReceiverTrait;

use std::sync::Arc;
use std::sync::Mutex;

/// The main struct responsible for the connection and events.
#[derive(Clone)]
pub struct ChatClient {
    nick: String,
    channel: String,
    sender: Arc<Mutex<Sender<WebSocketStream>>>,
    receiver: Arc<Mutex<Receiver<WebSocketStream>>>,
}

impl ChatClient {
    /// Creates a new connection to hack.chat.
    ///
    /// ```
    /// let mut chat = ChatClient::new("WikiBot", "programming");
    /// // Joins ?programming with the nick "WikiBot"
    /// ```
    pub fn new(nick: &str, channel: &str) -> ChatClient {
        let url = Url::parse("wss://hack.chat/chat-ws").unwrap();
        let request = Client::connect(url).unwrap();
        let response = request.send().unwrap();
        
        let client = response.begin();
        let (mut sender, receiver) = client.split();

        let join_packet = json!({
            "cmd": "join",
            "nick": nick,
            "channel": channel
        });
        let message = Message::text(join_packet.to_string());
        sender.send_message(&message).unwrap();

        return ChatClient {
            nick: nick.to_string(),
            channel: channel.to_string(),
            sender: Arc::new(Mutex::new(sender)),
            receiver: Arc::new(Mutex::new(receiver))
        };
    }

    /// Sends a message to the current channel.
    ///
    /// ```
    /// let mut chat = ChatClient::new("TestBot", "botDev");
    /// chat.send_message("Hello there people".to_string());
    /// ```
    ///
    /// ```
    /// let mut chat = ChatClient::new("TestBot", "botDev");
    ///
    /// let problem_count = 99;
    /// chat.send_message(format!("I got {} problems but Rust ain't one", problem_count));
    /// ```
    pub fn send_message(&mut self, message: String) {
        let chat_packet = json!({
            "cmd": "chat",
            "text": message
        });
        let message = Message::text(chat_packet.to_string());
        self.sender.lock().unwrap().send_message(&message).unwrap();
    }

    fn send_ping(&mut self) {
        let ping_packet = json!({
            "cmd": "ping"
        });
        let message = Message::text(ping_packet.to_string());
        self.sender.lock().unwrap().send_message(&message).unwrap();
    }

    /// Sends a stats request, which results in an Info event that has the number of connected
    /// IPs and channels.
    pub fn send_stats_request(&mut self) {
        let stats_packet = json!({
            "cmd": "stats"
        });
        let message = Message::text(stats_packet.to_string());
        self.sender.lock().unwrap().send_message(&message).unwrap();
    }

    /// Starts the ping thread, which sends regular pings to keep the connection open.
    pub fn start_ping_thread(&mut self) {
        let mut chat_clone = self.clone();
        thread::spawn(move|| {
            loop {
                thread::sleep_ms(60 * 1000);
                chat_clone.send_ping();
            }
        });
    }

    /// Returns an iterator of hack.chat events such as messages.
    ///
    /// #Examples
    /// ```
    /// let mut chat = ChatClient::new("GreetingBot", "botDev");
    /// chat.start_ping_thread(); //Start the ping thread so we keep connected
    ///
    /// for event in chat.iter() {
    ///     match event {
    ///         ChatEvent::JoinRoom(nick) => {
    ///             chat.send_message(format!("Welcome to the chat {}!", nick));
    ///         },
    ///         ChatEvent::LeaveRoom(nick) => {
    ///             chat.send_message(format!("Goodbye {}, see you later!", nick));
    ///         },
    ///         _ => {}
    ///     }
    /// }
    /// ```
    pub fn iter(&mut self) -> ChatClient {
        return self.clone();
    }
}

impl Iterator for ChatClient {
    type Item = ChatEvent;
    fn next(&mut self) -> Option<ChatEvent> {
        loop {
            let message: Message = match self.receiver.lock().unwrap().recv_message() {
                Ok(message) => message,
                Err(e) => {
                    println!("{}", e);
                    continue;
                }
            };
            
            match message.opcode {
                Type::Text => {
                    let data = std::str::from_utf8(&*message.payload).unwrap();
                    let cmdpacket: serde_json::Value = match serde_json::from_slice(&*message.payload) {
                        Ok(packet) => packet,
                        Err(e) => {
                            println!("{}", e);
                            continue;
                        }
                    };

                    match cmdpacket.get("cmd").unwrap_or(&serde_json::Value::Null).as_str() {
                        Some("chat") => {
                            let decodedpacket: ChatPacket = json::decode(&data).unwrap();
                            if decodedpacket.nick != self.nick {
                                return Some(ChatEvent::Message (
                                        decodedpacket.nick,
                                        decodedpacket.text,
                                        decodedpacket.trip.unwrap_or("".to_string())
                                        ));
                            }else {
                                continue;
                            }
                        },
                        Some("info") => {
                            let decodedpacket: InfoWarnPacket = json::decode(&data).unwrap();
                            return Some(ChatEvent::Info (
                                    decodedpacket.text
                                    ));
                        },
                        Some("onlineAdd") => {
                            let decodedpacket: OnlineChangePacket = json::decode(&data).unwrap();
                            return Some(ChatEvent::JoinRoom (
                                    decodedpacket.nick
                                    ));

                        },
                        Some("onlineRemove") => {
                            let decodedpacket: OnlineChangePacket = json::decode(&data).unwrap();
                            return Some(ChatEvent::LeaveRoom (
                                    decodedpacket.nick
                                    ));
                        },
                        _ => {
                            println!("Unsupported message type");
                            continue;
                        }
                    }
                },
                Type::Ping => {
                    self.sender.lock().unwrap().send_message(&Message::pong(message.payload)).unwrap();
                },
                _ => {
                    return None;
                }
            };
            return None;
        }
    }
}

/// Various Hack.chat events
pub enum ChatEvent {
    /// Raised when there is a new message from the channel
    ///
    /// The format is ChatEvent::Message(nick, text, trip_code)
    Message (String, String, String),
    /// Rasied when someone joins the channel
    ///
    /// The format is ChatEvent::JoinRoom(nick)
    JoinRoom (String),
    /// Raised when someone leaves the channel
    ///
    /// The format is ChatEvent::LeaveRoom(nick)
    LeaveRoom (String),
    /// Raised when there is an event from the channel itself.
    /// Some examples include:
    ///
    /// * The result of the stats requests
    /// * A user being banned.
    Info (String)
}

#[derive(RustcEncodable, RustcDecodable)]
struct GenericPacket {
    cmd: String
}

#[derive(RustcDecodable)]
struct ChatPacket {
    nick: String,
    text: String,
    trip: Option<String>
}

#[derive(RustcDecodable)]
struct OnlineChangePacket {
    nick: String
}

#[derive(RustcDecodable)]
struct InfoWarnPacket {
    text: String
}
