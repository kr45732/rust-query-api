/*
 * Rust Query API - A versatile API facade for the Hypixel Auction API
 * Copyright (c) 2022 kr45732
 *
 * This program is free software: you can redistribute it and/or modify
 * it under the terms of the GNU Affero General Public License as
 * published by the Free Software Foundation, either version 3 of the
 * License, or (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU Affero General Public License for more details.
 *
 * You should have received a copy of the GNU Affero General Public License
 * along with this program.  If not, see <https://www.gnu.org/licenses/>.
 */

use crate::config::Config;
use crate::{statics::*, structs::*};
use base64::engine::general_purpose;
use base64::Engine;
use dashmap::{DashMap, DashSet};
use deadpool_postgres::Client;
use futures::{pin_mut, Future};
use log::{error, info};
use postgres_types::{ToSql, Type};
use serde_json::Value;
use std::sync::{Arc, Mutex};
use std::time::UNIX_EPOCH;
use std::{fs::OpenOptions, thread, time::SystemTime};
use tokio::time::{self, Duration};
use tokio_postgres::{binary_copy::BinaryCopyInWriter, Error};

/* Repeat a task */
pub async fn start_auction_loop<F, Fut>(mut f: F)
where
    F: Send + 'static + FnMut() -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    // Create stream of intervals.
    let mut interval = time::interval(get_duration_until_api_update().await);
    tokio::spawn(async move {
        loop {
            // Skip tick at 0ms
            interval.tick().await;
            // Wait until next tick.
            interval.tick().await;
            // Spawn a task for this tick.
            f().await;
            // Updated to new interval
            interval = time::interval(get_duration_until_api_update().await);
        }
    });
}

/* Gets the time until the next API update according to Cloudflare headers */
async fn get_duration_until_api_update() -> Duration {
    let mut num_attempts = 0;
    loop {
        num_attempts += 1;
        let res = HTTP_CLIENT
            .get("https://api.hypixel.net/skyblock/auctions?page=0")
            .send()
            .await;
        match res {
            Ok(res_unwrap) => match res_unwrap.header("age") {
                Some(age_header) => {
                    let age = age_header.as_str().parse::<u64>().unwrap();

                    // Cloudfare doesn't return an exact time in ms, so the +2 accounts for that
                    let time = 60 - age + 2;

                    // Retry in 15 seconds if headers are giving weird values
                    if time > 120 {
                        thread::sleep(Duration::from_secs(15));
                        continue;
                    }

                    // Cannot return 0 duration
                    if time == 0 {
                        return Duration::from_millis(1);
                    }

                    return Duration::from_secs(time);
                }
                None => return Duration::from_millis(1),
            },
            Err(_) => {
                // Retry in 15 seconds
                thread::sleep(Duration::from_secs(15));
            }
        }
        if num_attempts == 15 {
            panic(String::from(
                "Failed 15 consecutive attempts to contact the Hypixel API",
            ));
        }
    }
}

/* Log and send an info message to the Discord webhook */
pub fn info(desc: String) {
    info_mention(desc, false);
}

pub fn info_mention(desc: String, mention: bool) {
    info!("{}", desc);
    tokio::spawn(async move {
        if let Some(webhook) = WEBHOOK.lock().await.as_ref() {
            let _ = webhook
                .send(|message| {
                    message.mention(mention).embed(|embed| {
                        embed
                            .title("Information")
                            .color(0x00FFFF)
                            .description(&desc)
                    })
                })
                .await;
        }
    });
}

/* Log and send an error message to the Discord webhook */
pub fn error(desc: String) {
    error!("{}", desc);
    tokio::spawn(async move {
        if let Some(webhook) = WEBHOOK.lock().await.as_ref() {
            let _ = webhook.send(|message| {
                message.embed(|embed| embed.title("Error").color(0xFF0000).description(&desc))
            });
        }
    });
}

/* Send a panic message to the Discord webhook and panic */
pub fn panic(desc: String) {
    tokio::spawn(async move {
        if let Some(webhook) = WEBHOOK.lock().await.as_ref() {
            let _ = webhook
                .send(|message| {
                    message.embed(|embed| {
                        embed
                            .title("Force Panic")
                            .color(0xFF0000)
                            .description(&desc)
                    })
                })
                .await;
        }

        panic!("{}", desc);
    });
}

pub fn parse_nbt(data: &str) -> Option<PartialNbt> {
    general_purpose::STANDARD
        .decode(data)
        .ok()
        .and_then(|bytes| nbt::from_gzip_reader::<_, PartialNbt>(std::io::Cursor::new(bytes)).ok())
}

pub fn calculate_with_taxes(price: f64) -> f64 {
    let price_float = price as f64;
    let tax_rate = if price >= 1000000.0 { 0.98 } else { 0.99 };

    price_float * tax_rate
}

pub fn valid_api_key(config: Arc<Config>, key: String, admin_only: bool) -> bool {
    if config.admin_api_key == key {
        return true;
    }
    if admin_only {
        return false;
    }
    config.api_key.is_empty() || (key == config.api_key)
}

pub fn update_lower_else_insert(id: &String, starting_bid: f64, prices: &DashMap<String, f64>) {
    if let Some(mut ele) = prices.get_mut(id) {
        if starting_bid < *ele {
            *ele = starting_bid;
        }
    } else {
        prices.insert(id.clone(), starting_bid);
    }
}

pub async fn update_query_database(
    mut auctions: Mutex<Vec<QueryDatabaseItem>>,
) -> Result<u64, Error> {
    let database = get_client().await;

    let _ = database.simple_query("TRUNCATE TABLE query").await;

    let copy_statement = database.prepare("COPY query FROM STDIN BINARY").await?;
    let copy_sink = database.copy_in(&copy_statement).await?;

    let copy_writer = BinaryCopyInWriter::new(
        copy_sink,
        &[
            Type::TEXT,
            Type::TEXT,
            Type::INT8,
            Type::TEXT,
            Type::TEXT,
            Type::TEXT,
            Type::INT8,
            Type::TEXT_ARRAY,
            Type::BOOL,
            BID_ARRAY.lock().await.to_owned().unwrap(),
            Type::INT4,
        ],
    );

    pin_mut!(copy_writer);

    // Write to copy sink
    for m in auctions.get_mut().unwrap().iter() {
        let row: Vec<&'_ (dyn ToSql + Sync)> = vec![
            &m.uuid,
            &m.auctioneer,
            &m.end_t,
            &m.item_name,
            &m.tier,
            &m.item_id,
            &m.starting_bid,
            &m.enchants,
            &m.bin,
            &m.bids,
            &m.count,
        ];

        copy_writer.as_mut().write(&row).await?;
    }

    copy_writer.finish().await
}

pub async fn update_pets_database(pet_prices: DashMap<String, AvgSum>) -> Result<u64, Error> {
    let database = get_client().await;

    // Add all old pet prices to the new prices if the new prices doesn't have that old pet name
    let old_pet_prices = database.query("SELECT * FROM pets", &[]).await?;
    for old_pet in old_pet_prices {
        let old_name: String = old_pet.get("name");
        let old_count: i32 = old_pet.get("count");
        let old_price: i64 = old_pet.get("price");
        let old_sum: i64 = old_price * (old_count as i64);

        if pet_prices.contains_key(&old_name) {
            pet_prices.alter(&old_name, |_, value| value.add_multiple(old_sum, old_count));
        } else {
            pet_prices.insert(
                old_name,
                AvgSum {
                    sum: old_sum,
                    count: old_count,
                },
            );
        }
    }

    let _ = database.simple_query("TRUNCATE TABLE pets").await;

    let copy_statement = database.prepare("COPY pets FROM STDIN BINARY").await?;
    let copy_sink = database.copy_in(&copy_statement).await?;
    let copy_writer = BinaryCopyInWriter::new(copy_sink, &[Type::TEXT, Type::INT8, Type::INT4]);
    pin_mut!(copy_writer);

    // Write to copy sink
    for m in pet_prices.iter() {
        copy_writer
            .as_mut()
            .write(&[
                m.key() as &(dyn ToSql + Sync),
                &m.value().get_average() as &(dyn ToSql + Sync),
                &m.value().count as &(dyn ToSql + Sync),
            ])
            .await?;
    }

    copy_writer.finish().await
}

pub async fn update_avg_ah_database(
    mut avg_ah_prices: Mutex<Vec<AvgAh>>,
    time_t: i64,
) -> Result<u64, Error> {
    let database = get_client().await;

    // Delete auctions older than 7 days
    tokio::spawn(async {
        let _ = get_client()
            .await
            .simple_query(
                &format!(
                    "DELETE FROM average WHERE time_t < {}",
                    (get_timestamp_millis() - Duration::from_secs(604800).as_millis())
                )
                .to_string(),
            )
            .await;
    });

    // Insert new average auctions
    database
        .execute(
            "INSERT INTO average VALUES ($1, $2)",
            &[&time_t, avg_ah_prices.get_mut().unwrap()],
        )
        .await
}

pub async fn update_avg_bin_database(
    mut avg_bin_prices: Mutex<Vec<AvgAh>>,
    time_t: i64,
) -> Result<u64, Error> {
    let database = get_client().await;

    // Delete bins older than 7 days
    tokio::spawn(async {
        let _ = get_client()
            .await
            .simple_query(
                &format!(
                    "DELETE FROM average_bin WHERE time_t < {}",
                    (get_timestamp_millis() - Duration::from_secs(604800).as_millis())
                )
                .to_string(),
            )
            .await;
    });

    // Insert new bins auctions
    database
        .execute(
            "INSERT INTO average_bin VALUES ($1, $2)",
            &[&time_t, avg_bin_prices.get_mut().unwrap()],
        )
        .await
}

pub async fn update_bins_local(bin_prices: &DashMap<String, f64>) -> Result<(), serde_json::Error> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("lowestbin.json")
        .unwrap();
    serde_json::to_writer(file, bin_prices)
}

pub async fn update_under_bins_local(
    bin_prices: &DashMap<String, Value>,
) -> Result<(), serde_json::Error> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("underbin.json")
        .unwrap();
    serde_json::to_writer(file, &bin_prices)
}

pub async fn update_query_items_local(query_prices: &Mutex<Vec<QueryDatabaseItem>>) {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("query_items.json")
        .unwrap();
    let lock = query_prices.lock().unwrap();
    let _ = serde_json::to_writer(
        file,
        &lock
            .iter()
            .map(|o| o.item_name.as_str())
            .collect::<DashSet<&str>>(),
    );
}

pub async fn get_client() -> Client {
    DATABASE.lock().await.as_ref().unwrap().get().await.unwrap()
}

pub fn get_timestamp_millis() -> u128 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_millis()
}
