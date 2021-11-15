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

use hyper::{
    header,
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use log::info;
use query_api::{api_handler::*, statics::*, structs::*, utils::*, webhook::Webhook};
use reqwest::Url;
use simplelog::*;
use std::{env, fmt::Write, fs::File};
use substring::Substring;
use tokio::time::Duration;
use tokio_postgres::NoTls;
use dotenv::dotenv;

/* Entry point to the program. Creates loggers, reads config, creates query table, starts auction loop and server */
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Check if debug or release build
    if cfg!(debug_assertions) {
        println!("Running a debug build");
    } else {
        println!("Running a release build");
    }

    // Create log files
    println!("Creating log files...");
    CombinedLogger::init(vec![
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
    .unwrap();
    println!("Loggers created.");

    // Read config
    println!("Reading config");
    dotenv().ok();
    let _ = BASE_URL
                .lock()
                .unwrap()
                .write_str(&env::var("BASE_URL").unwrap());
    let _ = PORT
                .lock()
                .unwrap()
                .write_str(&env::var("PORT").unwrap());
    let combined = format!("{}:{}", &env::var("BASE_URL").unwrap(), &env::var("PORT").unwrap());
    println!("{}", combined);
    let _ = URL
                .lock()
                .unwrap()
                .write_str(combined.as_str());
    let _ = POSTGRES_DB_URL
                .lock()
                .unwrap()
                .write_str(&env::var("POSTGRES_URL").unwrap());
    let _ = API_KEY
                .lock()
                .unwrap()
                .write_str(&env::var("API_KEY").unwrap());
    unsafe {
        let _ = WEBHOOK.insert(Webhook::from_url(&env::var("WEBHOOK_URL").unwrap()));
    }

    // Connect to database
    let (client, connection) =
        tokio_postgres::connect(POSTGRES_DB_URL.lock().unwrap().as_str(), NoTls).await?;
    tokio::spawn(async move {
        match connection.await {
            Ok(_) => {
                info("Successfully connected to database".to_string()).await;
            }
            Err(e) => {
                error(format!("Error connecting to database: {}", e)).await;
            }
        };
    });

    // Create the tables
    unsafe {
        let client = DATABASE.insert(client);
        // Drop the query table if exists
        let _ = client.simple_query("DROP TABLE IF EXISTS query").await;
        // Create new query table
        let _ = client
            .simple_query(
                "CREATE TABLE query (
                 uuid TEXT NOT NULL PRIMARY KEY,
                 auctioneer TEXT,
                 end_t BIGINT,
                 item_name TEXT,
                 tier TEXT,
                 item_id TEXT,
                 starting_bid BIGINT,
                 enchants TEXT[]
                )",
            )
            .await;

        // Drop the pets table if exists
        let _ = client.simple_query("DROP TABLE IF EXISTS pets").await;
        // Create new pets table
        let _ = client
            .simple_query(
                "CREATE TABLE pets (
                 name TEXT NOT NULL PRIMARY KEY,
                 price BIGINT
                )",
            )
            .await;
    }

    // Start the auction loop
    println!("Starting auction loop...");
    fetch_auctions().await;

    set_interval(
        || async {
            fetch_auctions().await;
        },
        Duration::from_millis(60000),
    );

    // Start the server
    println!("Starting server...");
    start_server().await;

    Ok(())
}

/* Starts the server listening on URL */
async fn start_server() {
    let server_address = URL.lock().unwrap().parse().unwrap();

    let make_service =
        make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(handle_response)) });

    let server = Server::bind(&server_address).serve(make_service);

    println!("Listening on http://{}", server_address);

    if let Err(e) = server.await {
        error(format!("Error when starting server: {}", e)).await;
    }
}

/* Handles http requests to the server */
async fn handle_response(req: Request<Body>) -> hyper::Result<Response<Body>> {
    info!("{} {}", req.method(), req.uri().path().substring(0, 30));

    if let (&Method::GET, "/") = (req.method(), req.uri().path()) {
        base()
    } else if let (&Method::GET, "/query") = (req.method(), req.uri().path()) {
        query(req).await
    } else if let (&Method::GET, "/pets") = (req.method(), req.uri().path()) {
        pets(req).await
    } else {
        not_found()
    }
}

async fn pets(req: Request<Body>) -> hyper::Result<Response<Body>> {
    let mut query = "".to_string();
    let mut key = "".to_string();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in Url::parse(
        &format!(
            "http://{}{}",
            URL.lock().unwrap(),
            &req.uri().to_string()
        )
        .to_string(),
    )
    .unwrap()
    .query_pairs()
    {
        if query_pair.0 == "query" {
            query = query_pair.1.to_string();
        } else if query_pair.0 == "key" {
            key = query_pair.1.to_string();
        }
    }

    // The API key in request doesn't match
    if key != API_KEY.lock().unwrap().as_str() {
        return bad_request("Not authorized");
    }

    if query.len() == 0 {
        return bad_request("The query paremeter cannot be empty");
    }

    unsafe {
        // Reference to the database
        let database_ref = DATABASE.as_ref();

        // Database isn't connected
        if database_ref.is_none() {
            return internal_error("Database isn't connected");
        }

        let results_cursor;
        // Find and sort using query JSON
        results_cursor = database_ref
            .unwrap()
            .query(
                &format!("SELECT * FROM pets WHERE name IN ({})", query),
                &[],
            )
            .await;

        if let Err(e) = results_cursor {
            // This shouldn't happen
            return internal_error(&format!("Error when querying database: {}", e).to_string());
        }

        // Convert the cursor iterator to a vector
        let mut results_vec = vec![];
        results_cursor.unwrap().into_iter().for_each(|ele| {
            results_vec.push(PetsDatabaseItem::from(ele));
        });

        // Return the vector of auctions serialized into JSON
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&results_vec).unwrap()))
            .unwrap())
    }
}

async fn query(req: Request<Body>) -> hyper::Result<Response<Body>> {
    // Query paremeters
    let mut query = "".to_string();
    let mut sort = "".to_string();
    let mut key = "".to_string();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in Url::parse(
        &format!(
            "http://{}{}",
            URL.lock().unwrap(),
            &req.uri().to_string()
        )
        .to_string(),
    )
    .unwrap()
    .query_pairs()
    {
        if query_pair.0 == "query" {
            query = query_pair.1.to_string();
        } else if query_pair.0 == "sort" {
            sort = query_pair.1.to_string();
        } else if query_pair.0 == "key" {
            key = query_pair.1.to_string();
        }
    }

    // The API key in request doesn't match
    if key != API_KEY.lock().unwrap().as_str() {
        return bad_request("Not authorized");
    }

    if query.len() == 0 {
        return bad_request("The query paremeter cannot be empty");
    }

    unsafe {
        // Reference to the database
        let database_ref = DATABASE.as_ref();

        // Database isn't connected
        if database_ref.is_none() {
            return internal_error("Database isn't connected");
        }

        let results_cursor;
        // Find and sort using query JSON
        if sort.is_empty() {
            results_cursor = database_ref
                .unwrap()
                .query(&format!("SELECT * FROM query WHERE {}", query), &[])
                .await;
        } else {
            results_cursor = database_ref
                .unwrap()
                .query(
                    &format!("SELECT * FROM query WHERE {} ORDER BY {}", query, sort),
                    &[],
                )
                .await;
        }

        if let Err(e) = results_cursor {
            // This shouldn't happen
            return internal_error(&format!("Error when querying database: {}", e).to_string());
        }

        // Convert the cursor iterator to a vector
        let mut results_vec = vec![];
        results_cursor.unwrap().into_iter().for_each(|ele| {
            results_vec.push(DatabaseItem::from(ele));
        });

        // Return the vector of auctions serialized into JSON
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&results_vec).unwrap()))
            .unwrap())
    }
}

fn base() -> hyper::Result<Response<Body>> {
    // Returns information & statistics about the API
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            "{{
            \"success\":true,
            \"query\":
            {{
                \"is_updating\":{},
                \"total_updates\":{},
                \"last_updated\":{}
            }}
        }}",
            *IS_UPDATING.lock().unwrap(),
            *TOTAL_UPDATES.lock().unwrap(),
            *LAST_UPDATED.lock().unwrap()
        )))
        .unwrap())
}
