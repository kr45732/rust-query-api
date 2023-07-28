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

use std::sync::Arc;
use std::{
    error::Error,
    fs::{self, File},
};

use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use dotenv::dotenv;
use simplelog::{CombinedLogger, LevelFilter, SimpleLogger, WriteLogger};
use tokio_postgres::NoTls;

use query_api::config::{Config, Feature};
use query_api::{
    api_handler::update_auctions,
    server::start_server,
    statics::{BID_ARRAY, DATABASE, WEBHOOK},
    utils::{info, start_auction_loop},
    webhook::Webhook,
};

/* Entry point to the program. Creates loggers, reads config, creates tables, starts auction loop and server */
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Read config
    println!("Reading config");
    if dotenv().is_err() {
        println!("Cannot find a .env file, will attempt to use environment variables");
    }
    let config = Arc::new(Config::load_or_panic());

    if config.debug {
        // Create log files
        CombinedLogger::init(vec![
            SimpleLogger::new(LevelFilter::Info, simplelog::Config::default()),
            WriteLogger::new(
                LevelFilter::Info,
                simplelog::Config::default(),
                File::create("info.log")?,
            ),
            WriteLogger::new(
                LevelFilter::Debug,
                simplelog::Config::default(),
                File::create("debug.log")?,
            ),
        ])
        .expect("Error when creating loggers");
        println!("Loggers Created");
    }

    if !config.webhook_url.is_empty() {
        let _ = WEBHOOK
            .lock()
            .await
            .insert(Webhook::from_url(config.webhook_url.as_str()));
    }

    if config.is_enabled(Feature::Query)
        || config.is_enabled(Feature::AverageAuction)
        || config.is_enabled(Feature::AverageBin)
        || config.is_enabled(Feature::Pets)
    {
        // Connect to database
        let database = DATABASE
            .lock()
            .await
            .insert(
                Pool::builder(Manager::from_config(
                    config.postgres_url.parse::<tokio_postgres::Config>()?,
                    NoTls,
                    ManagerConfig {
                        recycling_method: RecyclingMethod::Fast,
                    },
                ))
                .max_size(16)
                .runtime(Runtime::Tokio1)
                .build()?,
            )
            .get()
            .await?;

        if config.is_enabled(Feature::Query) {
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
                .insert(database.prepare("SELECT $1::_bid").await?.params()[0].clone());

            // Create query table if doesn't exist
            let _ = database
                .simple_query(
                    "CREATE UNLOGGED TABLE IF NOT EXISTS query (
                            uuid TEXT NOT NULL PRIMARY KEY,
                            auctioneer TEXT,
                            end_t BIGINT,
                            item_name TEXT,
                            lore TEXT,
                            tier TEXT,
                            item_id TEXT,
                            internal_id TEXT,
                            starting_bid BIGINT,
                            highest_bid BIGINT,
                            lowestbin_price REAL,
                            enchants TEXT[],
                            attributes TEXT[],
                            bin BOOLEAN,
                            bids bid[],
                            count SMALLINT,
                            potato_books SMALLINT,
                            stars SMALLINT,
                            farming_for_dummies SMALLINT,
                            transmission_tuner SMALLINT,
                            mana_disintegrator SMALLINT,
                            reforge TEXT,
                            rune TEXT,
                            skin TEXT,
                            power_scroll TEXT,
                            drill_upgrade_module TEXT,
                            drill_fuel_tank TEXT,
                            drill_engine TEXT,
                            dye TEXT,
                            accessory_enrichment TEXT,
                            recombobulated BOOLEAN,
                            wood_singularity BOOLEAN,
                            art_of_war BOOLEAN,
                            art_of_peace BOOLEAN,
                            etherwarp BOOLEAN,
                            necron_scrolls TEXT[],
                            gemstones TEXT[]
                        )",
                )
                .await?;
        }

        if config.is_enabled(Feature::AverageAuction) || config.is_enabled(Feature::AverageBin) {
            // Create avg_ah custom type
            let _ = database
                .simple_query(
                    "CREATE TYPE avg_ah AS (
                            price REAL,
                            sales REAL
                        )",
                )
                .await;

            if config.is_enabled(Feature::AverageAuction) {
                // Create average auction table if doesn't exist
                let _ = database
                    .simple_query(
                        "CREATE TABLE IF NOT EXISTS average_auction (
                                time_t INT,
                                item_id TEXT,
                                price REAL,
                                sales REAL,
                                PRIMARY KEY (time_t, item_id)
                            )",
                    )
                    .await?;

                let _ = database
                    .simple_query(
                        "CREATE INDEX IF NOT EXISTS average_auction_time_t_idx ON average_auction (time_t)",
                    )
                    .await?;
                let _ = database
                    .simple_query(
                        "CREATE INDEX IF NOT EXISTS average_auction_item_id_idx ON average_auction (item_id)",
                    )
                    .await?;
            }

            if config.is_enabled(Feature::AverageBin) {
                // Create average bins table if doesn't exist
                let _ = database
                    .simple_query(
                        "CREATE TABLE IF NOT EXISTS average_bin (
                                time_t INT,
                                item_id TEXT,
                                price REAL,
                                sales REAL,
                                PRIMARY KEY (time_t, item_id)
                            )",
                    )
                    .await?;

                let _ = database
                    .simple_query(
                        "CREATE INDEX IF NOT EXISTS average_bin_time_t_idx ON average_bin (time_t)",
                    )
                    .await?;
                let _ = database
                    .simple_query(
                        "CREATE INDEX IF NOT EXISTS average_bin_item_id_idx ON average_bin (item_id)",
                    )
                    .await?;
            }
        }

        if config.is_enabled(Feature::Pets) {
            // Create pets table if doesn't exist
            let _ = database
                .simple_query(
                    "CREATE TABLE IF NOT EXISTS pets (
                            name TEXT NOT NULL PRIMARY KEY,
                            price BIGINT,
                            count INTEGER
                        )",
                )
                .await?;
        }
    }

    if !config.disable_updating {
        // Remove any files from previous runs
        let _ = fs::remove_file("lowestbin.json");
        let _ = fs::remove_file("underbin.json");
        let _ = fs::remove_file("query_items.json");

        info(String::from("Starting auction loop..."));
        let auction_config = config.clone();
        start_auction_loop(move || {
            let auction_config = auction_config.clone();
            async move {
                loop {
                    let auction_config = auction_config.clone();
                    if update_auctions(auction_config).await {
                        break;
                    }
                }
            }
        })
        .await;
    }

    info(String::from("Starting server..."));
    start_server(config.clone()).await;

    Ok(())
}
