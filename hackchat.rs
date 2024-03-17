use serde_json::json;
use websocket::{ClientBuilder, Message, OwnedMessage};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::Duration;

struct HackChat {
    nick: String,
    channel: String,
    online_users: Vec<String>,
    on_message: Vec<Box<dyn Fn(&HackChat, &str, &str) + Send>>,
    on_whisper: Vec<Box<dyn Fn(&HackChat, &str, &str, &serde_json::Value) + Send>>,
    on_join: Vec<Box<dyn Fn(&HackChat, &str) + Send>>,
    on_leave: Vec<Box<dyn Fn(&HackChat, &str) + Send>>,
    ws: websocket::client::sync::Client<std::net::TcpStream>,
}

impl HackChat {
    fn new(nick: String, channel: String) -> HackChat {
        let ws = ClientBuilder::new("wss://hack.chat/chat-ws")
            .unwrap()
            .connect_insecure()
            .unwrap();

        let mut hack_chat = HackChat {
            nick,
            channel,
            online_users: Vec::new(),
            on_message: Vec::new(),
            on_whisper: Vec::new(),
            on_join: Vec::new(),
            on_leave: Vec::new(),
            ws,
        };

        hack_chat.send_packet(json!({"cmd": "join", "channel": &hack_chat.channel, "nick": &hack_chat.nick}));

        // Start ping thread
        let hack_chat = Arc::new(Mutex::new(hack_chat));
        let ping_hack_chat = Arc::clone(&hack_chat);
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_secs(60));
                let mut hack_chat = ping_hack_chat.lock().unwrap();
                hack_chat.send_packet(json!({"cmd": "ping"}));
            }
        });

        hack_chat
    }

    fn send_message(&self, msg: &str) {
        self.send_packet(json!({"cmd": "chat", "text": msg}));
    }

    fn send_to(&self, target: &str, msg: &str) {
        self.send_packet(json!({"cmd": "whisper", "nick": target, "text": msg}));
    }

    fn move_channel(&mut self, new_channel: &str) {
        self.channel = new_channel.to_string();
        self.send_packet(json!({"cmd": "move", "channel": new_channel}));
    }

    fn change_nick(&mut self, new_nick: &str) {
        self.nick = new_nick.to_string();
        self.send_packet(json!({"cmd": "changenick", "nick": new_nick}));
    }

    fn send_packet(&self, packet: serde_json::Value) {
        let encoded = packet.to_string();
        self.ws.send_message(&OwnedMessage::Text(encoded)).unwrap();
    }

    fn daemon(&self) {
        let hack_chat = Arc::new(Mutex::new(self.clone()));
        let daemon_hack_chat = Arc::clone(&hack_chat);
        thread::spawn(move || {
            let mut hack_chat = daemon_hack_chat.lock().unwrap();
            hack_chat.run();
        });
    }

    fn run(&mut self) {
        while let Ok(msg) = self.ws.recv_message() {
            match msg {
                Message::Text(text) => {
                    let result: HashMap<String, serde_json::Value> = serde_json::from_str(&text).unwrap();
                    match result.get("cmd").and_then(|cmd| cmd.as_str()) {
                        Some("chat") if result.get("nick").map_or(false, |nick| nick != &self.nick) => {
                            let text = result.get("text").unwrap().as_str().unwrap();
                            for handler in &self.on_message {
                                handler(self, text, result.get("nick").unwrap().as_str().unwrap());
                            }
                        }
                        Some("onlineAdd") => {
                            let nick = result.get("nick").unwrap().as_str().unwrap();
                            self.online_users.push(nick.to_string());
                            for handler in &self.on_join {
                                handler(self, nick);
                            }
                        }
                        Some("onlineRemove") => {
                            let nick = result.get("nick").unwrap().as_str().unwrap();
                            if let Some(pos) = self.online_users.iter().position(|x| x == nick) {
                                self.online_users.remove(pos);
                            }
                            for handler in &self.on_leave {
                                handler(self, nick);
                            }
                        }
                        Some("onlineSet") => {
                            if let Some(nicks) = result.get("nicks") {
                                if let Some(nicks) = nicks.as_array() {
                                    for nick in nicks {
                                        self.online_users.push(nick.as_str().unwrap().to_string());
                                    }
                                }
                            }
                        }
                        Some("info") if result.get("type").map_or(false, |t| t == "whisper") => {
                            let text = result.get("text").unwrap().as_str().unwrap();
                            let from = result.get("from").unwrap().as_str().unwrap();
                            for handler in &self.on_whisper {
                                handler(self, text, from, &result);
                            }
                        }
                        _ => {}
                    }
                }
                _ => {}
            }
        }
    }
}
