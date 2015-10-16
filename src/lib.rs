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
extern crate rustc_serialize;

use std::thread;

use rustc_serialize::json;

use websocket::{Client, Message, WebSocketStream};
use websocket::client::request::Url;
use websocket::client::sender::Sender;
use websocket::client::receiver::Receiver;

use websocket::ws::sender::Sender as SenderTrait;
use websocket::ws::receiver::Receiver as ReceiverTrait;

use std::sync::Arc;
use std::sync::Mutex;

#[derive(Clone)]
pub struct ChatClient {
    nick: String,
    channel: String,
    sender: Arc<Mutex<Sender<WebSocketStream>>>,
    receiver: Arc<Mutex<Receiver<WebSocketStream>>>,
}

impl ChatClient {
    /// Creates a new connection to hack.chat.
    pub fn new(nick: &str, channel: &str) -> ChatClient {
        let url = Url::parse("wss://hack.chat/chat-ws").unwrap();
        let request = Client::connect(url).unwrap();
        let response = request.send().unwrap();
        
        let client = response.begin();
        let (mut sender, receiver) = client.split();

        let join_packet = json::encode(&JoinPacket {
            cmd: "join".to_string(),
            nick: nick.to_string(),
            channel: channel.to_string()
        }).unwrap();
        let message = Message::Text(join_packet);
        sender.send_message(message).unwrap();

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
    pub fn send_message(&mut self, message: String) {
        let chat_packet = json::encode(&ChatPacketSend {
            cmd: "chat".to_string(),
            text: message
        }).unwrap();
        let message = Message::Text(chat_packet);
        self.sender.lock().unwrap().send_message(message).unwrap();
    }

    fn send_ping(&mut self) {
        let ping_packet = json::encode(&GenericPacket {
            cmd: "ping".to_string()
        }).unwrap();
        let message = Message::Text(ping_packet);
        self.sender.lock().unwrap().send_message(message).unwrap();
    }

    pub fn send_stats_request(&mut self) {
        let stats_packet = json::encode(&GenericPacket {
            cmd: "stats".to_string()
        }).unwrap();
        let message = Message::Text(stats_packet);
        self.sender.lock().unwrap().send_message(message).unwrap();
    }

    /// Starts the ping thread, whichs sends regular pings to keep the connection open.
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
    pub fn iter(&mut self) -> ChatClient {
        return self.clone();
    }
}

impl Iterator for ChatClient {
    type Item = ChatEvent;
    fn next(&mut self) -> Option<ChatEvent> {
        loop {
            let message = match self.receiver.lock().unwrap().recv_message() {
                Ok(message) => message,
                Err(e) => {
                    println!("{}", e);
                    continue;
                }
            };
            match message {
                Message::Text(data) => {
                    let cmdpacket: GenericPacket = match json::decode(&data) {
                        Ok(cmdpacket) => cmdpacket,
                        Err(e) => {
                            println!("{}", e);
                            continue;
                        }
                    };

                    let cmd = cmdpacket.cmd;
                    if cmd == "chat" {
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
                    }else if cmd == "info" {
                        let decodedpacket: InfoWarnPacket = json::decode(&data).unwrap();
                        return Some(ChatEvent::Info (
                            decodedpacket.text
                        ));
                    }else if cmd == "onlineAdd" {
                        let decodedpacket: OnlineChangePacket = json::decode(&data).unwrap();
                        return Some(ChatEvent::JoinRoom (
                            decodedpacket.nick
                        ));
                    }else if cmd == "onlineRemove" {
                        let decodedpacket: OnlineChangePacket = json::decode(&data).unwrap();
                        return Some(ChatEvent::LeaveRoom (
                            decodedpacket.nick
                        ));
                    }else {
                        continue;
                    }
                },
                Message::Ping(data) => {
                    self.sender.lock().unwrap().send_message(Message::Pong(data)).unwrap();
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

#[derive(RustcEncodable)]
struct JoinPacket {
    cmd: String,
    channel: String,
    nick: String
}

#[derive(RustcEncodable)]
struct ChatPacketSend {
    cmd: String,
    text: String
}
