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

use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use dotenv::dotenv;
use query_api::{api_handler::*, server::start_server, statics::*, utils::*, webhook::Webhook};
use simplelog::*;
use std::{
    env,
    error::Error,
    fmt::Write,
    fs::{self, File},
};
use tokio_postgres::NoTls;

/* Entry point to the program. Creates loggers, reads config, creates tables, starts auction loop and server */
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create log files
    CombinedLogger::init(vec![
        SimpleLogger::new(LevelFilter::Info, Config::default()),
        WriteLogger::new(
            LevelFilter::Info,
            Config::default(),
            File::create("info.log").unwrap(),
        ),
        WriteLogger::new(
            LevelFilter::Debug,
            Config::default(),
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
    let _ = BASE_URL
        .lock()
        .await
        .write_str(&env::var("BASE_URL").expect("Unable to find BASE_URL environment variable"));
    let _ = PORT
        .lock()
        .await
        .write_str(&env::var("PORT").expect("Unable to find PORT environment variable"));
    let _ = URL.lock().await.write_str(
        format!(
            "{}:{}",
            &env::var("BASE_URL").unwrap(),
            &env::var("PORT").unwrap()
        )
        .as_str(),
    );
    let _ = POSTGRES_DB_URL.lock().await.write_str(
        &env::var("POSTGRES_URL").expect("Unable to find POSTGRES_URL environment variable"),
    );
    let _ = API_KEY
        .lock()
        .await
        .write_str(&env::var("API_KEY").expect("Unable to find API_KEY environment variable"));
    let _ = ADMIN_API_KEY
        .lock()
        .await
        .write_str(&env::var("ADMIN_API_KEY").unwrap_or(API_KEY.lock().await.to_string()));
    for feature in env::var("FEATURES")
        .expect("Unable to find FEATURES environment variable")
        .split("+")
    {
        match feature {
            "QUERY" => *ENABLE_QUERY.lock().await = true,
            "PETS" => *ENABLE_PETS.lock().await = true,
            "LOWESTBIN" => *ENABLE_LOWESTBIN.lock().await = true,
            "UNDERBIN" => {
                if *ENABLE_LOWESTBIN.lock().await {
                    *ENABLE_UNDERBIN.lock().await = true
                } else {
                    panic!("LOWESTBIN must be enabled BEFORE enabling UNDERBIN");
                }
            }
            "AVERAGE_AUCTION" => *ENABLE_AVERAGE_AUCTION.lock().await = true,
            _ => panic!("Invalid feature type: {}", feature),
        }
    }

    let _ = WEBHOOK.lock().await.insert(Webhook::from_url(
        &env::var("WEBHOOK_URL").expect("Unable to find WEBHOOK_URL environment variable"),
    ));

    // Connect to database
    let database = DATABASE
        .lock()
        .await
        .insert(
            Pool::builder(Manager::from_config(
                POSTGRES_DB_URL
                    .lock()
                    .await
                    .as_str()
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
    start_auction_loop(|| async {
        update_auctions().await;
    })
    .await;

    info("Starting server...".to_string());
    start_server().await;

    Ok(())
}
