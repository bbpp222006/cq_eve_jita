mod util;
use crossbeam::channel::{bounded, select, Receiver, Sender};
use regex::Regex;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::env;
use std::thread;
use std::time::Duration;

fn main() {
    thread::sleep(Duration::from_secs(5)); //延时5s启动
    let ws_url =env::var("WS").unwrap();
    // let ws_url = "ws://192.168.3.230:20002/"; //"ws://10.243.159.138:30010";

    let (socket_send_tx, message_out) = util::create_socket_channel(&ws_url);
    let (update_sig_tx, update_sig_rx) = bounded(1);
    let (item_search_tx, item_search_rx): (
        Sender<std::string::String>,
        Receiver<std::string::String>,
    ) = bounded(10);
    let (search_result_tx, search_result_rx) = bounded(10);

    let update_sig_cron = update_sig_tx.clone();
    let cron_loop = thread::spawn(move || {
        //定时更新循环
        loop {
            thread::sleep(Duration::from_secs(60 * 60 * 24)); //一天更新一次
            println!("触发定时更新");
            update_sig_cron.send(1).unwrap();
        }
    });

    let item_search_db_rx = item_search_rx.clone();
    let database_loop = thread::spawn(move || {
        //数据库查询和更新模块
        let mut db_hash = HashMap::new();
        loop {
            select! {
                recv(update_sig_rx) -> _ =>{
                    println!("收到更新信号，将进行数据库更新");
                    db_hash =util::update_db(49);

                },
                recv(item_search_db_rx) -> item =>{
                    let name = item.unwrap();
                    println!("收到查询信号，查询目标{}",name);
                    let names = util::get_name(&db_hash, &name);
                    let price = util::get_price(names);
                    // let return_str = util::pretty_str(price);
                    search_result_tx.send(price).unwrap();
                }

            }
        }
    });

    let message_out_jita = message_out.clone();
    let socket_send_tx_jita = socket_send_tx.clone();
    let jita_re = Regex::new(r"^jita +([^ ]+)( all)?").unwrap();
    let jita_loop = thread::spawn(move || {
        // jita模块
        loop {
            let raw_message = message_out_jita.recv().unwrap();
            let v: Value = serde_json::from_str(&raw_message).unwrap();
            println!("{}", v);

            if let Some(v_) = jita_re.captures(v["message"].as_str().map_or("", |x| x)) {
                let user_id = v["user_id"].as_u64().unwrap();
                let group_id = v["group_id"].as_u64().unwrap();

                let item_name = v_.get(1).unwrap().as_str();
                let all_flag = v_.get(2).is_some();
                println!("{} {}", item_name, all_flag);
                let mut str_to_send = String::new();

                if all_flag {
                    item_search_tx.send(item_name.to_owned()).unwrap();
                    let price_vec = search_result_rx.recv().unwrap();
                    str_to_send =
                        util::pretty_str(price_vec).map_or("可能真的搜不到？？".to_owned(), |x| x);
                } else {
                    item_search_tx.send(item_name.to_owned()).unwrap();
                    let price_vec = util::filter_price(search_result_rx.recv().unwrap());
                    str_to_send = util::pretty_str(price_vec).map_or(
                        r#"未查询到相关物品在售卖,或只查询到涂装,请检查名称输入是否有误
若要查看所有结果 在命令后加all即可,例如:jita 三钛合金 all"#
                            .to_owned(),
                        |x| x,
                    );
                }

                let message_to_send = json!({
                    "action": "send_group_msg",
                    "params": {
                        "group_id": group_id,
                        "message": str_to_send,
                    },
                    "echo": "123"
                })
                .to_string();
                socket_send_tx_jita.send(message_to_send).unwrap();
            } else {
                println!("没有匹配到")
            }
        }
    });

    let update_sig_setup = update_sig_tx.clone();
    update_sig_setup.send(1).unwrap(); //第一次更新

    println!("启动成功");
    let _ = jita_loop.join();
    let _ = cron_loop.join();
    let _ = database_loop.join();
    println!("Exited");
}
