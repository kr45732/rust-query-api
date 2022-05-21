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

use crate::{
    config::Config,
    statics::{BID_ARRAY, DATABASE},
};
use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio_postgres::NoTls;

pub async fn init_database(config: Config) {
    let database = DATABASE
        .lock()
        .await
        .insert(
            Pool::builder(Manager::from_config(
                config
                    .postgres_url
                    .parse::<tokio_postgres::Config>()
                    .unwrap(),
                NoTls,
                ManagerConfig {
                    recycling_method: RecyclingMethod::Fast,
                },
            ))
            .max_size(16)
            .runtime(Runtime::Tokio1)
            .build()
            .unwrap(),
        )
        .get()
        .await
        .unwrap();

    // Create bid custom type
    let _ = database
        .simple_query(
            "CREATE TYPE bid AS (
                    bidder TEXT,
                    amount BIGINT
                )",
        )
        .await;

    // Get the bid array type and store for future use
    let _ = BID_ARRAY
        .lock()
        .await
        .insert(database.prepare("SELECT $1::_bid").await.unwrap().params()[0].clone());

    // Create avg_ah custom type
    let _ = database
        .simple_query(
            "CREATE TYPE avg_ah AS (
                    item_id TEXT,
                    price DOUBLE PRECISION,
                    sales REAL
                )",
        )
        .await;

    // Create query table if doesn't exist
    let _ = database
        .simple_query(
            "CREATE TABLE IF NOT EXISTS query (
                    uuid TEXT NOT NULL PRIMARY KEY,
                    auctioneer TEXT,
                    end_t BIGINT,
                    item_name TEXT,
                    tier TEXT,
                    item_id TEXT,
                    starting_bid BIGINT,
                    enchants TEXT[],
                    bin BOOLEAN,
                    bids bid[]
                )",
        )
        .await;

    // Create pets table if doesn't exist
    let _ = database
        .simple_query(
            "CREATE TABLE IF NOT EXISTS pets (
                    name TEXT NOT NULL PRIMARY KEY,
                    price BIGINT
                )",
        )
        .await;

    // Create average auction table if doesn't exist
    let _ = database
        .simple_query(
            "CREATE TABLE IF NOT EXISTS average (
                    time_t BIGINT NOT NULL PRIMARY KEY,
                    prices avg_ah[]
                )",
        )
        .await;
}
