use deadpool_postgres::{Manager, ManagerConfig, Pool, RecyclingMethod, Runtime};
use tokio_postgres::NoTls;
use crate::statics::{BID_ARRAY, DATABASE};


// let database = DATABASE
    //     .lock()
    //     .await
    //     .insert(
    //         Pool::builder(Manager::from_config(
    //             config
    //                 .postgres_url
    //                 .parse::<tokio_postgres::Config>()
    //                 .unwrap(),
    //             NoTls,
    //             ManagerConfig {
    //                 recycling_method: RecyclingMethod::Fast,
    //             },
    //         ))
    //         .max_size(16)
    //         .runtime(Runtime::Tokio1)
    //         .build()
    //         .unwrap(),
    //     )
    //     .get()
    //     .await
    //     .unwrap();

    // // Create bid custom type
    // let _ = database
    //     .simple_query(
    //         "CREATE TYPE bid AS (
    //                 bidder TEXT,
    //                 amount BIGINT
    //             )",
    //     )
    //     .await;

    // // Get the bid array type and store for future use
    // let _ = BID_ARRAY
    //     .lock()
    //     .await
    //     .insert(database.prepare("SELECT $1::_bid").await.unwrap().params()[0].clone());

    // // Create avg_ah custom type
    // let _ = database
    //     .simple_query(
    //         "CREATE TYPE avg_ah AS (
    //                 item_id TEXT,
    //                 price DOUBLE PRECISION,
    //                 sales REAL
    //             )",
    //     )
    //     .await;

    // // Create query table if doesn't exist
    // let _ = database
    //     .simple_query(
    //         "CREATE TABLE IF NOT EXISTS query (
    //                 uuid TEXT NOT NULL PRIMARY KEY,
    //                 auctioneer TEXT,
    //                 end_t BIGINT,
    //                 item_name TEXT,
    //                 tier TEXT,
    //                 item_id TEXT,
    //                 starting_bid BIGINT,
    //                 enchants TEXT[],
    //                 bin BOOLEAN,
    //                 bids bid[]
    //             )",
    //     )
    //     .await;

    // // Create pets table if doesn't exist
    // let _ = database
    //     .simple_query(
    //         "CREATE TABLE IF NOT EXISTS pets (
    //                 name TEXT NOT NULL PRIMARY KEY,
    //                 price BIGINT
    //             )",
    //     )
    //     .await;

    // // Create average auction table if doesn't exist
    // let _ = database
    //     .simple_query(
    //         "CREATE TABLE IF NOT EXISTS average (
    //                 time_t BIGINT NOT NULL PRIMARY KEY,
    //                 prices avg_ah[]
    //             )",
    //     )
    //     .await;
