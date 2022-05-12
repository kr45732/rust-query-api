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

#![deny(unsafe_code)]
#![warn(
    clippy::all,
    clippy::await_holding_lock,
    clippy::char_lit_as_u8,
    clippy::checked_conversions,
    clippy::dbg_macro,
    clippy::debug_assert_with_mut_call,
    clippy::doc_markdown,
    clippy::empty_enum,
    clippy::enum_glob_use,
    clippy::exit,
    clippy::expl_impl_clone_on_copy,
    clippy::explicit_deref_methods,
    clippy::explicit_into_iter_loop,
    clippy::fallible_impl_from,
    clippy::filter_map_next,
    clippy::flat_map_option,
    clippy::float_cmp_const,
    clippy::fn_params_excessive_bools,
    clippy::from_iter_instead_of_collect,
    clippy::if_let_mutex,
    clippy::implicit_clone,
    clippy::imprecise_flops,
    clippy::inefficient_to_string,
    clippy::invalid_upcast_comparisons,
    clippy::large_digit_groups,
    clippy::large_stack_arrays,
    clippy::large_types_passed_by_value,
    clippy::let_unit_value,
    clippy::linkedlist,
    clippy::lossy_float_literal,
    clippy::macro_use_imports,
    clippy::manual_ok_or,
    clippy::map_err_ignore,
    clippy::map_flatten,
    clippy::map_unwrap_or,
    clippy::match_on_vec_items,
    clippy::match_same_arms,
    clippy::match_wild_err_arm,
    clippy::match_wildcard_for_single_variants,
    clippy::mem_forget,
    clippy::mismatched_target_os,
    clippy::missing_enforced_import_renames,
    clippy::mut_mut,
    clippy::mutex_integer,
    clippy::needless_borrow,
    clippy::needless_continue,
    clippy::needless_for_each,
    clippy::option_option,
    clippy::path_buf_push_overwrite,
    clippy::ptr_as_ptr,
    clippy::rc_mutex,
    clippy::ref_option_ref,
    clippy::rest_pat_in_fully_bound_structs,
    clippy::same_functions_in_if_condition,
    clippy::semicolon_if_nothing_returned,
    clippy::single_match_else,
    clippy::string_add_assign,
    clippy::string_add,
    clippy::string_lit_as_bytes,
    clippy::string_to_string,
    clippy::todo,
    clippy::trait_duplication_in_bounds,
    clippy::unimplemented,
    clippy::unnested_or_patterns,
    clippy::unused_self,
    clippy::useless_transmute,
    clippy::verbose_file_reads,
    clippy::zero_sized_map_values,
    future_incompatible,
    nonstandard_style,
    rust_2018_idioms
)]

use std::sync::Arc;
use std::{
    error::Error,
    fs::{self, File},
};

use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use dotenv::dotenv;
use simplelog::{CombinedLogger, LevelFilter, SimpleLogger, WriteLogger};
use tokio_postgres::NoTls;

use query_api::config::Config;
use query_api::{api_handler::*, server::start_server, statics::*, utils::*, webhook::Webhook};

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
    let _ = WEBHOOK
        .lock()
        .await
        .insert(Webhook::from_url(config.webhook_url.as_str()));
    // Connect to database
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
