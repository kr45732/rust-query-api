/*
 * Rust Query API - A versatile API facade for the Hypixel Auction API
 * Copyright (c) 2021 kr45732
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

use crate::{statics::*, structs::*};
use chrono::prelude::{DateTime, Utc};
use dashmap::DashMap;
use futures::{pin_mut, Future};
use hyper::{header, Body, Response, StatusCode};
use log::{error, info};
use postgres_types::{ToSql, Type};
use std::{fs::OpenOptions, result::Result as StdResult, time::SystemTime};
use tokio::time::{self, Duration};
use tokio_postgres::{binary_copy::BinaryCopyInWriter, Error};

/* 400 */
pub fn bad_request(reason: &str) -> hyper::Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            "{{\"success\":false,\"reason\":\"{}\"}}",
            reason
        )))
        .unwrap())
}

/* 404 */
pub fn not_found() -> hyper::Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{\"success\":false,\"reason\":\"Not found\"}"))
        .unwrap())
}

/* 500 */
pub fn internal_error(reason: &str) -> hyper::Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            "{{\"success\":false,\"reason\":\"{}\"}}",
            reason
        )))
        .unwrap())
}

/* Repeat a task */
pub fn set_interval<F, Fut>(mut f: F, dur: Duration)
where
    F: Send + 'static + FnMut() -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    // Create stream of intervals.
    let mut interval = time::interval(dur);
    tokio::spawn(async move {
        // Skip the first tick at 0ms.
        interval.tick().await;
        loop {
            // Wait until next tick.
            interval.tick().await;
            // Spawn a task for this tick.
            f().await;
        }
    });
}

pub async fn info(desc: String) {
    info!("{}", desc);
        let _ = WEBHOOK
            .as_ref()
            .unwrap()
            .send(|message| {
                message.embed(|embed| {
                    embed
                        .title("Information")
                        .url(&format!("http://{}", &URL.lock().unwrap()).to_string())
                        .color(0x00FFFF)
                        .description(&desc)
                        .timestamp(&get_discord_timestamp())
                })
            })
            .await;
}

pub async fn error(desc: String) {
    error!("{}", desc);
        let _ = WEBHOOK
            .as_ref()
            .unwrap()
            .send(|message| {
                message.embed(|embed| {
                    embed
                        .title("Error")
                        .url(&format!("http://{}", &URL.lock().unwrap()).to_string())
                        .color(0xFF0000)
                        .description(&desc)
                        .timestamp(&get_discord_timestamp())
                })
            })
            .await;
}

pub async fn panic(desc: String) {
    error!("{}", desc);
        let _ = WEBHOOK
            .as_ref()
            .unwrap()
            .send(|message| {
                message.embed(|embed| {
                    embed
                        .title("Force panic")
                        .url(&format!("http://{}", &URL.lock().unwrap()).to_string())
                        .color(0xFF0000)
                        .description(&desc)
                        .timestamp(&get_discord_timestamp())
                })
            })
            .await;
    panic!("{}", desc);
}

fn get_discord_timestamp() -> String {
    let dt: DateTime<Utc> = SystemTime::now().into();
    format!("{}", dt.format("%+"))
}

pub fn to_nbt(item_bytes: ItemBytes) -> Result<PartialNbt, Box<dyn std::error::Error>> {
    let bytes: StdResult<Vec<u8>, _> = item_bytes.into();
    let nbt: PartialNbt = nbt::from_gzip_reader(std::io::Cursor::new(bytes?))?;
    Ok(nbt)
}

pub async fn update_query_database(auctions: Vec<DatabaseItem>) -> Result<u64, Error> {
        let database = DATABASE.as_ref().unwrap();
        let _ = database.simple_query("TRUNCATE TABLE query").await;

        let copy_statement = database
            .prepare("COPY query FROM STDIN BINARY")
            .await
            .unwrap();
        let copy_sink = database.copy_in(&copy_statement).await.unwrap();
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
            ],
        );
        pin_mut!(copy_writer);

        // Write to copy sink
        let mut row: Vec<&'_ (dyn ToSql + Sync)> = Vec::new();
        for m in &auctions {
            row.clear();
            row.push(&m.uuid);
            row.push(&m.auctioneer);
            row.push(&m.end_t);
            row.push(&m.item_name);
            row.push(&m.tier);
            row.push(&m.item_id);
            row.push(&m.starting_bid);
            row.push(&m.enchants);
            copy_writer.as_mut().write(&row).await.unwrap();
        }

        copy_writer.finish().await
}

pub async fn update_pets_database(pet_prices: &mut DashMap<String, i64>) -> Result<u64, Error> {
        let database = DATABASE.as_ref().unwrap();

        // Add all old pet prices to the new prices if the new prices doesn't have that old pet name
        let old_pet_prices = database.query("SELECT * FROM pets", &[]).await.unwrap();
        for old_price in old_pet_prices {
            let old_price_name: String = old_price.get("name");
            let mut new_has = false;
            for new_price in pet_prices.iter_mut() {
                if old_price_name == *new_price.key() {
                    new_has = true;
                    break;
                }
            }
            if !new_has {
                pet_prices.insert(old_price_name, old_price.get("price"));
            }
        }

        let _ = database.simple_query("TRUNCATE TABLE pets").await;

        let copy_statement = database
            .prepare("COPY pets FROM STDIN BINARY")
            .await
            .unwrap();
        let copy_sink = database.copy_in(&copy_statement).await.unwrap();
        let copy_writer = BinaryCopyInWriter::new(copy_sink, &[Type::TEXT, Type::INT8]);
        pin_mut!(copy_writer);

        // Write to copy sink
        for m in pet_prices.iter() {
            copy_writer
                .as_mut()
                .write(&[
                    m.key() as &(dyn ToSql + Sync),
                    m.value() as &(dyn ToSql + Sync),
                ])
                .await
                .unwrap();
        }

        copy_writer.finish().await
}

pub async fn update_bins_local(bin_prices: &DashMap<String, i64>) -> Result<(), simd_json::Error> {
    let file = OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .open("lowestbin.json")
        .unwrap();
    simd_json::to_writer(file, bin_prices)
}
