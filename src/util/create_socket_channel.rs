use crossbeam::channel::{bounded, select, Receiver, Sender};
use serde_json::Value;
use std::{thread, time::Duration};
use websocket::{client::ClientBuilder, Message, OwnedMessage};

pub fn create_socket_channel(addr: &str) -> (Sender<String>, Receiver<String>) {
    let client = ClientBuilder::new(addr)
        .unwrap()
        .add_protocol("rust-websocket")
        .connect_insecure()
        .unwrap();

    println!("Successfully connected");

    let (mut receiver, mut sender) = client.split().unwrap();

    let (heartbeat_in, heartbeat_out) = bounded(10);
    let (message_in, message_out) = bounded(10);
    let (socket_send_tx, socket_send_rx): (
        Sender<std::string::String>,
        Receiver<std::string::String>,
    ) = bounded(10);

    thread::spawn(move || {
        // 接收消息线程
        for message in receiver.incoming_messages() {
            if let OwnedMessage::Text(msg) = message.unwrap() {
                let v: Value = serde_json::from_str(&msg).unwrap();
                if v["meta_event_type"] == "heartbeat" {
                    // println!("心跳包加入通道！");
                    heartbeat_in.send(msg).unwrap();
                } else {
                    // println!("消息加入通道！");
                    message_in.send(msg).unwrap();
                }
            }
        }
    });

    thread::spawn(move || {
        // 发送消息线程
        loop {
            let meesage_send = socket_send_rx.recv().unwrap();
            println!("发送{}", meesage_send);
            sender.send_message(&Message::text(meesage_send)).unwrap();
        }
    });

    thread::spawn(move || {
        // 检测心跳是否正常，不正常则尝试重连
        loop {
            select! {
                recv(heartbeat_out) -> _ =>  {
                    ()
                },
                default(Duration::from_secs(600)) => {
                    panic!("10分钟没有检测到心跳，程序退出")
                },
            }
        }
    });

    (socket_send_tx, message_out)
}
