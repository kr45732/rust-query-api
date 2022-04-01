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

use std::{
    error::Error,
    fs::{self, File},
};
use std::sync::Arc;

use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use dotenv::dotenv;
use simplelog::{CombinedLogger, LevelFilter, SimpleLogger, WriteLogger};
use tokio_postgres::NoTls;

use query_api::{api_handler::*, server::start_server, statics::*, utils::*, webhook::Webhook};
use query_api::config::Config;

/* Entry point to the program. Creates loggers, reads config, creates tables, starts auction loop and server */
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create log files
    CombinedLogger::init(vec![
        SimpleLogger::new(LevelFilter::Info, Default::default()),
        WriteLogger::new(
            LevelFilter::Info,
            Default::default(),
            File::create("info.log").unwrap(),
        ),
        WriteLogger::new(
            LevelFilter::Debug,
            Default::default(),
            File::create("debug.log").unwrap(),
        ),
    ])
        .expect("Error when creating loggers");
    println!("Loggers Created");

    // Read config
    println!("Reading config");
    if dotenv().is_err() {
        println!("Cannot find a .env file, will attempt to use environment variables");
    }

    let config = Arc::new(Config::load_or_panic());
    let _ = WEBHOOK.lock().await.insert(Webhook::from_url(config.webhook_url.as_str()));
    // Connect to database
    let database = DATABASE
        .lock()
        .await
        .insert(
            Pool::builder(Manager::from_config(
                config.postgres_url
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

    // Remove any files from previous runs
    let _ = fs::remove_file("lowestbin.json");
    let _ = fs::remove_file("underbin.json");
    let _ = fs::remove_file("query_items.json");

    info("Starting auction loop...".to_string());
    let auction_config = config.clone();
    start_auction_loop(move || {
        let auction_config = auction_config.clone();
        async move {
            update_auctions(auction_config).await;
        }
    })
        .await;

    info("Starting server...".to_string());
    start_server(config.clone()).await;

    Ok(())
}
