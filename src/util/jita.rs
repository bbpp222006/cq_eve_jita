use crossbeam::channel::{bounded, select, Receiver, Sender};
use levenshtein::levenshtein;
use num_format::{Locale, ToFormattedString};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;

pub fn update_db(all_page: u64) -> HashMap<String, u64> {
    let (hashmap_tx, hashmap_rx) = bounded(40);

    let mut db_hash = HashMap::new();

    thread::spawn(move || {
        for page in 1..all_page + 1 {
            let hashmap_tx_ = hashmap_tx.clone();

            thread::spawn(move || {
                let hashmap = id_to_name(get_type_page(page));
                hashmap_tx_.send(hashmap).unwrap();
                println!("{}页请求完成", page)
            });

            thread::sleep(Duration::from_secs_f32(0.1));
        }
    });

    for _ in 1..all_page + 1 {
        let new_db_hash = hashmap_rx.recv().unwrap();
        db_hash.extend(new_db_hash);
    }

    db_hash
}

fn id_to_name(id_vec: Vec<u64>) -> HashMap<String, u64> {
    let client = reqwest::blocking::Client::new();
    let res = client
        .post("https://esi.evepc.163.com/latest/universe/names/?datasource=serenity")
        .json(&id_vec)
        .send()
        .unwrap()
        .text()
        .unwrap();

    let v: Value = serde_json::from_str(&res).unwrap();

    let aval_vac: HashMap<String, u64> = v
        .as_array()
        .unwrap()
        .into_iter()
        .map(|value| {
            (
                value["name"].as_str().unwrap().to_string(),
                value["id"].as_u64().unwrap(),
            )
        })
        .collect();

    aval_vac
}

fn get_type_page(page: u64) -> Vec<u64> {
    let client = reqwest::blocking::Client::new();
    let url = format!(
        "https://esi.evepc.163.com/latest/universe/types/?datasource=serenity&page={}",
        page
    );
    let res = client.get(url).send().unwrap().text().unwrap();

    let v: Value = serde_json::from_str(&res).unwrap();

    let return_vec: Vec<u64> = v
        .as_array()
        .unwrap()
        .into_iter()
        .map(|a| a.as_u64().unwrap())
        .collect();
    return_vec
}

fn get_score(a: &str, b: &str) -> u64 {
    let lst = levenshtein(a, b) as f64;
    let word_len = {
        if a.chars().count() > b.chars().count() {
            a.chars().count() as f64
        } else {
            b.chars().count() as f64
        }
    };

    let a_score = ((word_len - lst) / word_len * 100.0) as u64;
    a_score
}

pub fn get_name(all_item_dic: &HashMap<String, u64>, name: &str) -> Vec<(String, u64)> {
    let mut b: Vec<(String, u64)> = all_item_dic
        .into_iter()
        .map(|item| (item.0.to_string(), get_score(item.0, name)))
        .collect();

    b.sort_by(|a, b| b.1.cmp(&a.1));

    let mut c = vec![];

    // let mut num = 0;

    for target_name in b.iter() {
        if target_name.1 < 20 {
            break;
        } else {
            // num += 1;
            c.push((target_name.0.to_owned(), all_item_dic[&target_name.0]));
        }
        println!("{} {}", target_name.0, target_name.1);
    }
    c
}

pub fn get_price(item_vec: Vec<(String, u64)>) -> Vec<(String, (f64, f64))> {
    let client = reqwest::blocking::Client::new();
    let mut return_vec = vec![];
    let mut num = 0;
    for item in item_vec.into_iter() {
        if num > 10 {
            break;
        } else {
            let url = format!(
                "https://www.ceve-market.org/api/market/region/10000002/type/{}.json",
                item.1
            );
            let res = client.get(url).send().unwrap().text().unwrap();
            let v: Value = serde_json::from_str(&res).unwrap();
            let sell = v["sell"]["min"].as_f64().unwrap();
            let buy = v["buy"]["max"].as_f64().unwrap();
            if sell == 0.0 {
                continue;
            }
            return_vec.push((item.0, (sell, buy)));
            num += 1;
        }
    }
    return_vec
}

pub fn pretty_str(price_vec: Vec<(String, (f64, f64))>) -> String {
    let mut whole_sell = 0;
    let mut whole_buy = 0;

    let mut return_str = String::from("名称：卖/买\n");

    for item in price_vec.into_iter() {
        if item.1 .0 == 0.0 {
            continue;
        } else {
            whole_sell += item.1 .0 as u64;
            whole_buy += item.1 .1 as u64;
            if item.1 .0 < 1000.0 {
                return_str.push_str(&format!("{} {}/{}\n", &item.0, item.1 .0, item.1 .0));
                continue;
            } else {
                let sell = (item.1 .0 as u64).to_formatted_string(&Locale::en);
                let buy = (item.1 .1 as u64).to_formatted_string(&Locale::en);
                return_str.push_str(&format!("{} {}/{}\n", &item.0, sell, buy));
            }
        }
    }
    return_str.push_str(&format!(
        "统计： {}/{}\n",
        whole_sell.to_formatted_string(&Locale::en),
        whole_buy.to_formatted_string(&Locale::en)
    ));
    return_str
}
