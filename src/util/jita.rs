extern crate strsim;

use crossbeam::channel::{bounded, select, Receiver, Sender};
use num_format::{Locale, ToFormattedString};
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::thread;
use std::time::Duration;
use strsim::jaro;

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

// serde解析json
#[derive(Deserialize, Debug)]
struct Item {
    id: u64,
    name: String,
    category: String,
}

fn id_to_name(id_vec: Vec<u64>) -> HashMap<String, u64> {
    let client = reqwest::blocking::Client::new();

    fn send_batch_request(batch: &[u64]) -> Vec<Item> {
        let client = reqwest::blocking::Client::new(); // 创建同步的Reqwest客户端
        let url = "https://ali-esi.evepc.163.com/latest/universe/names/?datasource=serenity";


        let mut res = String::new();
        for _ in 0..10 {
            match client.post(url).json(batch).send() {
                Ok(res_) => {
                    res = res_.text().unwrap();
                    break;
                }
                Err(_) => {
                    println!("请求失败，重试");
                    thread::sleep(Duration::from_secs_f32(1.0));
                }                   
            }
        }

        // let res = client.post(url).json(batch).send().unwrap().text().unwrap();

        // 将结果解析成您需要的数据结构，这里假设是String
        let parsed_result: Vec<Item> = serde_json::from_str(&res).unwrap();
        parsed_result
    }
    // 对id_vec 每100个分次请求，再合并为res
    let batch_size = 1000; // 设置每个批次的大小
    let mut combined_result = Vec::new();
    for batch in id_vec.chunks(batch_size) {
        // println!("id_name请求完成");
        let res = send_batch_request(batch);
        combined_result.extend(res);
    }

    let aval_vac: HashMap<String, u64> = combined_result
        .into_iter()
        .map(|value| (value.name, value.id))
        .collect();

    aval_vac
}

fn get_type_page(page: u64) -> Vec<u64> {
    let client = reqwest::blocking::Client::new();
    let url = format!(
        "https://ali-esi.evepc.163.com/latest/universe/types/?datasource=serenity&page={}",
        page
    );

    let mut res = String::new();
    for _ in 0..10 {
        match client.get(url.clone()).send() {
            Ok(res_) => {
                res = res_.text().unwrap();
                break;
            }
            Err(_) => {
                println!("请求失败，重试");
                thread::sleep(Duration::from_secs_f32(1.0));
            }                   
        }
    }
    // let res = client.get(url).send().unwrap().text().unwrap();

    let v: Value = serde_json::from_str(&res).unwrap();

    let return_vec: Vec<u64> = v
        .as_array()
        .unwrap()
        .into_iter()
        .map(|a| a.as_u64().unwrap())
        .collect();
    return_vec
}

#[test]
fn test_get_type_page() {
    let db_hash = update_db(51);
    let name = "三钛合金";
    let names = get_name(&db_hash, name);
    let price = get_price(names);
    println!("{:?}", price);
}

fn get_score(dic_name: &str, search_name: &str) -> u64 {
    let mut pattern = dic_name.to_owned();
    let mut score = 0;

    fn circle_string(string: &mut String) {
        let a = string.pop().unwrap();
        string.insert(0, a);
    }

    for i in 0..dic_name.chars().count() - 1 {
        let temp_score = (jaro(&pattern, search_name) * 100.0) as u64;
        if temp_score >= score {
            score = temp_score
        }
        circle_string(&mut pattern);
    }

    // let word_len = a.chars().count() +b.chars().count() ;
    // let a_score =fuzz::partial_ratio("this is a test", "this is a test!");

    // let a_score = ((word_len - lst) / word_len * 100) as u64;
    score
}
//名称  匹配分数  id
pub fn get_name(all_item_dic: &HashMap<String, u64>, name: &str) -> Vec<(String, u64, u64)> {
    let mut b: Vec<(String, u64, u64)> = all_item_dic
        .into_iter()
        .map(|item| {
            (
                item.0.to_string(),
                get_score(item.0, name),
                all_item_dic[item.0],
            )
        })
        .collect();

    b.sort_by(|a, b| b.1.cmp(&a.1));
    b
    // let mut c = vec![];

    // let mut current_score = b.get(0).map_or(0, |a| a.1);
    // let mut max_num = 0;

    // for target_name in b.iter() {
    //     let score_gap = current_score - target_name.1;
    //     println!("{} {}", target_name.0, target_name.1);
    //     if score_gap > 5 || max_num > 50 {
    //         break;
    //     } else {
    //         max_num += 1;
    //         c.push((target_name.0.to_owned(), all_item_dic[&target_name.0]));
    //         current_score = target_name.1;
    //     }
    // }

    // // c.sort_by(|a, b| b.1.cmp(&a.1));
    // c
}
//名称  score  id            //名称   sell  buy  score
pub fn get_price(item_vec: Vec<(String, u64, u64)>) -> Vec<(String, (f64, f64, u64))> {
    let client = reqwest::blocking::Client::new();
    let mut return_vec = vec![];
    let mut num = 0;
    let mut try_num = 0;
    for (item_name, score, id) in item_vec.into_iter() {
        println!("{},{},{}", item_name, score, id);
        if num > 10 || score < 60 || try_num > 30 {
            break;
        } else {
            // 如果id=29668,将其改为44992
            let id = if id == 29668 { 44992 } else { id };
            let url = format!(
                "https://www.ceve-market.org/api/market/region/10000002/type/{}.json",
                id
            );
            try_num += 1;
            let res = client.get(url).send().unwrap().text().unwrap();
            let v: Value = serde_json::from_str(&res).unwrap();
            let sell = v["sell"]["min"].as_f64().unwrap();
            let buy = v["buy"]["max"].as_f64().unwrap();
            return_vec.push((item_name, (sell, buy, score)));
            if sell == 0.0 {
                continue;
            }
            num += 1;
        }
    }
    return_vec
}

pub fn pretty_str(price_vec: Vec<(String, (f64, f64, u64))>) -> Option<String> {
    let mut whole_sell = 0.0;
    let mut whole_buy = 0.0;
    if price_vec.len() == 0 {
        return None;
    }

    let mut return_str = String::from("名称：卖/买\n");

    for (num, (item_name, (sell, buy, score))) in price_vec.into_iter().enumerate() {
        if num > 10 {
            break;
        }
        whole_sell += sell;
        whole_buy += buy;
        let mut sellstr = sell.to_string();
        let mut buystr = buy.to_string();
        if sell > 1000.0 {
            sellstr = (sell as u64).to_formatted_string(&Locale::en);
        }
        if buy > 1000.0 {
            buystr = (buy as u64).to_formatted_string(&Locale::en);
        }
        return_str.push_str(&format!("{} {}/{}\n", item_name, sellstr, buystr));
    }
    return_str.push_str(&format!(
        "统计： {}/{}",
        (whole_sell as u64).to_formatted_string(&Locale::en),
        (whole_buy as u64).to_formatted_string(&Locale::en)
    ));

    Some(return_str)
}

pub fn filter_price(price_vec: Vec<(String, (f64, f64, u64))>) -> Vec<(String, (f64, f64, u64))> {
    let mut return_vec = vec![];
    let tuzhuang_reg = Regex::new(r"涂装").unwrap();
    let mut current_score = price_vec.get(0).map_or(0, |a| a.1 .2);
    // let mut max_num = 0;
    for (item_name, (sell, buy, score)) in price_vec.into_iter() {
        println!("{},{},{},{}", item_name, sell, buy, score);
        let score_gap = current_score - score;
        if score_gap > 9 {
            break;
        }
        if (tuzhuang_reg.is_match(&item_name) && score < 90) || (sell == 0.0 && score < 70) {
            continue;
        }

        current_score = score;
        return_vec.push((item_name, (sell, buy, score)));
        if current_score > 98 {
            break;
        }
    }
    return_vec
}
