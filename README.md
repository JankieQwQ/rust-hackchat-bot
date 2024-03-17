# rust-hackchat-bot
Rust Powered Hack-Chat bot.

Demo:
```Rust
fn main() {
    let mut hack_chat = HackChat::new("rust_bot".to_string(), "programming".to_string());
    hack_chat.on_message.push(Box::new(|chat, msg, nick| {
        println!("[{}]: {}", nick, msg);
    }));
    hack_chat.on_whisper.push(Box::new(|chat, msg, from, _| {
        println!("Whisper from [{}]: {}", from, msg);
    }));
    hack_chat.on_join.push(Box::new(|chat, nick| {
        println!("[{}] joined the chat", nick);
    }));
    hack_chat.on_leave.push(Box::new(|chat, nick| {
        println!("[{}] left the chat", nick);
    }));
    hack_chat.daemon();
    hack_chat.send_message("Hello, everyone!");
    loop {}
}
