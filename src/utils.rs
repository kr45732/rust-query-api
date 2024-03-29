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

use crate::{config::Config, statics::*, structs::*};
use base64::{engine::general_purpose, Engine};
use dashmap::{DashMap, DashSet};
use deadpool_postgres::Client;
use futures::{pin_mut, Future};
use log::{error, info};
use postgres_types::{ToSql, Type};
use serde_json::Value;
use std::{
    cmp::Ordering,
    fmt::Write,
    fs::OpenOptions,
    sync::{Arc, Mutex},
    thread,
    time::{Instant, SystemTime, UNIX_EPOCH},
};
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

        if let Ok(res) = HTTP_CLIENT
            .get("https://api.hypixel.net/skyblock/auctions?page=0")
            .send()
            .await
        {
            match res.headers().get("age") {
                Some(age_header) => {
                    let age = age_header.to_str().unwrap().parse::<u64>().unwrap();

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
            }
        }

        if num_attempts % 5 == 0 {
            error(format!(
                "Failed {num_attempts} consecutive attempts to contact the Hypixel API. Retrying in a minute.",
            ));
            thread::sleep(Duration::from_secs(60));
        } else {
            thread::sleep(Duration::from_secs(15));
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
            let _ = webhook
                .send(|message| {
                    message.embed(|embed| embed.title("Error").color(0xFF0000).description(&desc))
                })
                .await;
        }
    });
}

pub fn parse_nbt(data: &str) -> Option<PartialNbt> {
    general_purpose::STANDARD
        .decode(data)
        .ok()
        .and_then(|bytes| nbt::from_gzip_reader::<_, PartialNbt>(std::io::Cursor::new(bytes)).ok())
}

pub fn calculate_with_taxes(price: f32) -> f32 {
    let mut tax = 0.0;

    // 1% for claiming bin over 1m (when buying)
    if price >= 1000000.0 {
        tax += 0.01;
    }

    // Tax for starting new bin (when reselling)
    if price <= 10000000.0 {
        tax += 0.01;
    } else if price <= 100000000.0 {
        tax += 0.02;
    } else {
        tax += 0.025;
    }

    price * (1.0 - tax)
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

pub fn update_lower_else_insert(id: &str, starting_bid: f32, prices: &DashMap<String, f32>) {
    if let Some(mut ele) = prices.get_mut(id) {
        if starting_bid < *ele {
            *ele = starting_bid;
        }
    } else {
        prices.insert(id.to_string(), starting_bid);
    }
}

pub async fn update_query_bin_underbin_fn(
    auctions: Mutex<Vec<QueryDatabaseItem>>,
    ended_auction_uuids: DashSet<String>,
    is_full_update: bool,
    bin_prices: &DashMap<String, f32>,
    update_lowestbin: bool,
    last_updated: i64,
    update_underbin: bool,
    under_bin_prices: &DashMap<String, Value>,
) -> (String, String) {
    let mut ok_logs = String::new();
    let mut err_logs = String::new();

    let query_started = Instant::now();
    let _ = match update_query_database(
        auctions,
        ended_auction_uuids,
        is_full_update,
        bin_prices,
        update_lowestbin,
        last_updated,
    )
    .await
    {
        Ok(rows) => write!(
            ok_logs,
            "\nSuccessfully inserted {} query auctions into database in {}ms",
            rows,
            query_started.elapsed().as_millis()
        ),
        Err(e) => write!(err_logs, "\nError inserting query into database: {}", e),
    };

    if update_lowestbin {
        let bins_started = Instant::now();
        let _ = match update_bins_local(bin_prices).await {
            Ok(_) => write!(
                ok_logs,
                "\nSuccessfully updated bins file in {}ms",
                bins_started.elapsed().as_millis()
            ),
            Err(e) => write!(err_logs, "\nError updating bins file: {}", e),
        };

        if update_underbin {
            let under_bins_started = Instant::now();
            let _ = match update_under_bins_local(under_bin_prices).await {
                Ok(_) => write!(
                    ok_logs,
                    "\nSuccessfully updated under bins file in {}ms",
                    under_bins_started.elapsed().as_millis()
                ),
                Err(e) => {
                    write!(err_logs, "\nError updating under bins file: {}", e)
                }
            };
        }
    }

    (ok_logs, err_logs)
}

pub async fn update_pets_fn(pet_prices: DashMap<String, AvgSum>) -> (String, String) {
    let pets_started = Instant::now();
    match update_pets_database(pet_prices).await {
        Ok(rows) => (
            format!(
                "\nSuccessfully inserted {} pets into database in {}ms",
                rows,
                pets_started.elapsed().as_millis()
            ),
            String::new(),
        ),
        Err(e) => (
            String::new(),
            format!("\nError inserting pets into database: {}", e),
        ),
    }
}

pub async fn update_average_fn(
    name: &str,
    table: &str,
    avg_prices: DashMap<String, AvgSum>,
    time_t: i64,
) -> (String, String) {
    let avg_started = Instant::now();
    match update_avgerage_database(table, avg_prices, (time_t / 1000) as i32).await {
        Ok(count) => (
            format!(
                "\nSuccessfully inserted {} {} into database in {}ms",
                count,
                name,
                avg_started.elapsed().as_millis()
            ),
            String::new(),
        ),
        Err(e) => (
            String::new(),
            format!("\nError inserting {} into database: {}", name, e),
        ),
    }
}

async fn update_query_database(
    mut auctions: Mutex<Vec<QueryDatabaseItem>>,
    ended_auction_uuids: DashSet<String>,
    is_full_update: bool,
    bin_prices: &DashMap<String, f32>,
    update_lowestbin: bool,
    last_updated: i64,
) -> Result<u64, Error> {
    let database = get_client().await;

    if is_full_update {
        let _ = database.simple_query("TRUNCATE TABLE query").await?;

        let query_names = auctions
            .lock()
            .unwrap()
            .iter()
            .map(|o| o.item_name.to_string())
            .collect::<DashSet<String>>();
        update_query_items_local(query_names);
    } else {
        // Remove ended auctions and duplicate 'new' auctions
        let mut delete_uuids = ended_auction_uuids
            .iter()
            .map(|u| format!("'{}'", *u))
            .collect::<Vec<String>>();
        for ele in auctions.get_mut().unwrap().iter() {
            delete_uuids.push(format!("'{}'", ele.uuid));
        }

        if delete_uuids.is_empty() {
            let _ = database
                .simple_query(&format!(
                    "DELETE FROM query WHERE end_t <= {}",
                    last_updated
                ))
                .await?;
        } else {
            let _ = database
                .simple_query(&format!(
                    "DELETE FROM query WHERE uuid in ({}) OR end_t <= {}",
                    delete_uuids.join(","),
                    last_updated
                ))
                .await?;
        }
    }

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
            Type::TEXT,
            Type::TEXT,
            Type::INT8,
            Type::INT8,
            Type::FLOAT4,
            Type::TEXT_ARRAY,
            Type::TEXT_ARRAY,
            Type::BOOL,
            BID_ARRAY.lock().await.to_owned().unwrap(),
            Type::INT2,
            Type::INT2,
            Type::INT2,
            Type::INT2,
            Type::INT2,
            Type::INT2,
            Type::TEXT,
            Type::TEXT,
            Type::TEXT,
            Type::TEXT,
            Type::TEXT,
            Type::TEXT,
            Type::TEXT,
            Type::TEXT,
            Type::TEXT,
            Type::BOOL,
            Type::BOOL,
            Type::BOOL,
            Type::BOOL,
            Type::BOOL,
            Type::TEXT_ARRAY,
            Type::TEXT_ARRAY,
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
            &m.lore,
            &m.tier,
            &m.item_id,
            &m.internal_id,
            &m.starting_bid,
            &m.highest_bid,
            &m.lowestbin_price,
            &m.enchants,
            &m.attributes,
            &m.bin,
            &m.bids,
            &m.count,
            &m.potato_books,
            &m.stars,
            &m.farming_for_dummies,
            &m.transmission_tuner,
            &m.mana_disintegrator,
            &m.reforge,
            &m.rune,
            &m.skin,
            &m.power_scroll,
            &m.drill_upgrade_module,
            &m.drill_fuel_tank,
            &m.drill_engine,
            &m.dye,
            &m.accessory_enrichment,
            &m.recombobulated,
            &m.wood_singularity,
            &m.art_of_war,
            &m.art_of_peace,
            &m.etherwarp,
            &m.necron_scrolls,
            &m.gemstones,
        ];

        copy_writer.as_mut().write(&row).await?;
    }

    let rows_added = copy_writer.finish().await?;

    if !is_full_update {
        let query_names: DashSet<String> = DashSet::new();

        let mut all_auctions_sql = String::from("SELECT item_name");
        // These fields are only needed to update lowest bin
        if update_lowestbin {
            all_auctions_sql.push_str(", internal_id, lowestbin_price, bin");
        }
        all_auctions_sql.push_str(" FROM query");

        let all_auctions = database.query(&all_auctions_sql, &[]).await?;
        for ele in all_auctions {
            query_names.insert(ele.get("item_name"));

            // Has to be updated over all auctions instead of comparing previous lowest bins with new auctions
            if update_lowestbin && ele.get("bin") {
                let internal_id: String = ele.get("internal_id");
                let lowestbin_price: f32 = ele.get("lowestbin_price");
                update_lower_else_insert(&internal_id, lowestbin_price, bin_prices);
            }
        }

        update_query_items_local(query_names);
    }

    Ok(rows_added)
}

async fn update_pets_database(pet_prices: DashMap<String, AvgSum>) -> Result<u64, Error> {
    let database = get_client().await;

    // Add all old pet prices to the new prices if the new prices doesn't have that old pet name
    let old_pet_prices = database.query("SELECT * FROM pets", &[]).await?;
    for old_pet in old_pet_prices {
        let old_name: String = old_pet.get("name");
        let old_count: i32 = old_pet.get("count");
        let old_price: i64 = old_pet.get("price");
        let old_sum: i64 = old_price * (old_count as i64);

        if let Some(mut value) = pet_prices.get_mut(&old_name) {
            value.update(old_sum, old_count);
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
            .write(&[m.key(), &m.value().get_average(), &m.value().count])
            .await?;
    }

    copy_writer.finish().await
}

async fn update_avgerage_database(
    table: &str,
    avg_prices: DashMap<String, AvgSum>,
    time_t: i32, // In seconds
) -> Result<u64, Error> {
    let table_str = table.to_string();
    let database = get_client().await;

    // Delete averages older than 7 days
    tokio::spawn(async move {
        let _ = get_client()
            .await
            .simple_query(
                &format!(
                    "DELETE FROM {} WHERE time_t < {}",
                    table_str,
                    time_t - 604800 // 7 days (in seconds)
                )
                .to_string(),
            )
            .await;
    });

    // Insert new averages
    let copy_statement = database
        .prepare(&format!("COPY {} FROM STDIN BINARY", table))
        .await?;
    let copy_sink = database.copy_in(&copy_statement).await?;
    let copy_writer = BinaryCopyInWriter::new(
        copy_sink,
        &[Type::INT4, Type::TEXT, Type::FLOAT4, Type::FLOAT4],
    );
    pin_mut!(copy_writer);

    // Average all and write to copy
    for ele in avg_prices {
        copy_writer
            .as_mut()
            .write(&[
                &time_t,
                &ele.0,
                &(ele.1.sum as f32 / ele.1.count as f32),
                &(ele.1.count as f32),
            ])
            .await?;
    }

    copy_writer.finish().await
}

async fn update_bins_local(bin_prices: &DashMap<String, f32>) -> Result<(), serde_json::Error> {
    // Calculate lowestbin of item (regardless of attributes)
    let additional_prices = DashMap::new();
    for ele in bin_prices {
        if ele.key().contains("+ATTRIBUTE_SHARD_") {
            update_lower_else_insert(
                ele.key().split("+ATTRIBUTE_SHARD_").next().unwrap(),
                *ele.value(),
                &additional_prices,
            );
        }
    }
    for ele in additional_prices {
        bin_prices.insert(ele.0, ele.1);
    }

    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("lowestbin.json")
        .unwrap();
    serde_json::to_writer(file, bin_prices)
}

async fn update_under_bins_local(
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

fn update_query_items_local(query_prices: DashSet<String>) {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("query_items.json")
        .unwrap();
    let _ = serde_json::to_writer(file, &query_prices);
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

pub fn get_timestamp_secs() -> i32 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i32
}

pub fn is_false(b: &bool) -> bool {
    !b
}

/// Trust me, this is not overkill
pub fn median(data: &[f32]) -> f32 {
    match data.len() {
        even if even % 2 == 0 => {
            let fst = select(data, (even / 2) - 1);
            let snd = select(data, even / 2);

            (fst + snd) / 2.0
        }
        odd => select(data, odd / 2),
    }
}

fn select(data: &[f32], k: usize) -> f32 {
    let (left, pivot, right) = partition(data);

    let pivot_idx = left.len();

    match pivot_idx.cmp(&k) {
        Ordering::Equal => pivot,
        Ordering::Greater => select(&left, k),
        Ordering::Less => select(&right, k - (pivot_idx + 1)),
    }
}

fn partition(data: &[f32]) -> (Vec<f32>, f32, Vec<f32>) {
    let (pivot_slice, tail) = data.split_at(1);
    let pivot = pivot_slice[0];
    let (left, right) = tail.iter().fold((vec![], vec![]), |mut splits, next| {
        {
            let (ref mut left, ref mut right) = &mut splits;
            if next < &pivot {
                left.push(*next);
            } else {
                right.push(*next);
            }
        }
        splits
    });

    (left, pivot, right)
}

pub fn update_average_map(map: &DashMap<String, AvgSum>, id: &str, price: i64, count: i16) {
    // If the map already has this id, then add to the existing elements, otherwise create a new entry
    if let Some(mut value) = map.get_mut(id) {
        value.update(price, count as i32);
    } else {
        map.insert(
            id.to_string(),
            AvgSum {
                sum: price,
                count: count as i32,
            },
        );
    }
}
