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

use dotenv::dotenv;
use hyper::{
    header,
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use log::info;
use query_api::{api_handler::*, statics::*, structs::*, utils::*, webhook::Webhook};
use reqwest::Url;
use simplelog::*;
use std::{
    env,
    error::Error,
    fmt::Write,
    fs::{self, File},
};
use substring::Substring;
use tokio::time::Duration;
use tokio_postgres::NoTls;

/* Entry point to the program. Creates loggers, reads config, creates query table, starts auction loop and server */
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
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
    .expect("Error when creating loggers");
    println!("Loggers created.");

    // Read config
    println!("Reading config");
    dotenv().ok();
    let _ = BASE_URL
        .lock()
        .unwrap()
        .write_str(&env::var("BASE_URL").expect("Unable to find BASE_URL environment variable"));
    let _ = PORT
        .lock()
        .unwrap()
        .write_str(&env::var("PORT").expect("Unable to find PORT environment variable"));
    let _ = URL.lock().unwrap().write_str(
        format!(
            "{}:{}",
            &env::var("BASE_URL").unwrap(),
            &env::var("PORT").unwrap()
        )
        .as_str(),
    );
    let _ = POSTGRES_DB_URL.lock().unwrap().write_str(
        &env::var("POSTGRES_URL").expect("Unable to find POSTGRES_URL environment variable"),
    );
    let _ = API_KEY
        .lock()
        .unwrap()
        .write_str(&env::var("API_KEY").expect("Unable to find API_KEY environment variable"));
    for feature in env::var("FEATURES")
        .expect("Unable to find FEATURES environment variable")
        .split("+")
    {
        match feature {
            "QUERY" => *ENABLE_QUERY.lock().unwrap() = true,
            "PETS" => *ENABLE_PETS.lock().unwrap() = true,
            "LOWESTBIN" => *ENABLE_LOWESTBIN.lock().unwrap() = true,
            _ => panic!("Invalid feature type: {}", feature),
        }
    }
    unsafe {
        let _ = WEBHOOK.insert(Webhook::from_url(
            &env::var("WEBHOOK_URL").expect("Unable to find WEBHOOK_URL environment variable"),
        ));
    }

    // Connect to database
    let (client, connection) =
        tokio_postgres::connect(POSTGRES_DB_URL.lock().unwrap().as_str(), NoTls)
            .await
            .unwrap();
    tokio::spawn(async move {
        match connection.await {
            Ok(_) => {
                info("Successfully connected to database".to_string()).await;
            }
            Err(e) => {
                panic(format!("Error connecting to database: {}", e)).await;
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
    update_api().await;

    set_interval(
        || async {
            update_api().await;
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
        if *ENABLE_QUERY.lock().unwrap() {
            query(req).await
        } else {
            bad_request("Query feature is not enabled")
        }
    } else if let (&Method::GET, "/pets") = (req.method(), req.uri().path()) {
        if *ENABLE_PETS.lock().unwrap() {
            pets(req).await
        } else {
            bad_request("Pets feature is not enabled")
        }
    } else if let (&Method::GET, "/lowestbin") = (req.method(), req.uri().path()) {
        if *ENABLE_LOWESTBIN.lock().unwrap() {
            lowestbin(req).await
        } else {
            bad_request("Lowest bins feature is not enabled")
        }
    } else {
        not_found()
    }
}

async fn pets(req: Request<Body>) -> hyper::Result<Response<Body>> {
    let mut query = "".to_string();
    let mut key = "".to_string();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in
        Url::parse(&format!("http://{}{}", URL.lock().unwrap(), &req.uri().to_string()).to_string())
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
    let mut limit = "".to_string();
    let mut key = "".to_string();
    let mut item_name = "".to_string();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in
        Url::parse(&format!("http://{}{}", URL.lock().unwrap(), &req.uri().to_string()).to_string())
            .unwrap()
            .query_pairs()
    {
        if query_pair.0 == "query" {
            query = query_pair.1.to_string()
        } else if query_pair.0 == "sort" {
            sort = query_pair.1.to_string();
        } else if query_pair.0 == "limit" {
            limit = query_pair.1.to_string();
        } else if query_pair.0 == "key" {
            key = query_pair.1.to_string();
        } else if query_pair.0 == "name" {
            item_name = query_pair.1.to_string();
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
        // Database isn't connected
        if DATABASE.as_ref().is_none() {
            return internal_error("Database isn't connected");
        }

        // Reference to the database
        let database_ref = DATABASE.as_ref().unwrap();

        let results_cursor;
        // Find and sort using query JSON
        if sort.is_empty() {
            if item_name.is_empty() {
                if limit.is_empty() {
                    results_cursor = database_ref
                        .query(&format!("SELECT * FROM query WHERE {}", query), &[])
                        .await;
                } else {
                    results_cursor = database_ref
                        .query(
                            &format!("SELECT * FROM query WHERE {} LIMIT {}", query, limit),
                            &[],
                        )
                        .await;
                }
            } else {
                if limit.is_empty() {
                    results_cursor = database_ref
                        .query(
                            &format!("SELECT * FROM query WHERE item_name ILIKE $1 AND {}", query),
                            &[&item_name],
                        )
                        .await;
                } else {
                    results_cursor = database_ref
                        .query(
                            &format!(
                                "SELECT * FROM query WHERE item_name ILIKE $1 AND {} LIMIT {}",
                                query, limit
                            ),
                            &[&item_name],
                        )
                        .await;
                }
            }
        } else {
            if item_name.is_empty() {
                if limit.is_empty() {
                    results_cursor = database_ref
                        .query(
                            &format!("SELECT * FROM query WHERE {} ORDER BY {}", query, sort),
                            &[],
                        )
                        .await;
                } else {
                    results_cursor = database_ref
                        .query(
                            &format!(
                                "SELECT * FROM query WHERE {} ORDER BY {} LIMIT {}",
                                query, sort, limit
                            ),
                            &[],
                        )
                        .await;
                }
            } else {
                if limit.is_empty() {
                    results_cursor = database_ref
                        .query(
                            &format!(
                                "SELECT * FROM query WHERE item_name ILIKE $1 AND {} ORDER BY {}",
                                query, sort
                            ),
                            &[&item_name],
                        )
                        .await;
                } else {
                    results_cursor = database_ref
                    .query(
                        &format!(
                            "SELECT * FROM query WHERE item_name ILIKE $1 AND {} ORDER BY {} LIMIT {}",
                            query, sort, limit
                        ),
                        &[&item_name],
                    )
                    .await;
                }
            }
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

async fn lowestbin(req: Request<Body>) -> hyper::Result<Response<Body>> {
    // Query paremeters
    let mut key = "".to_string();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in
        Url::parse(&format!("http://{}{}", URL.lock().unwrap(), &req.uri().to_string()).to_string())
            .unwrap()
            .query_pairs()
    {
        if query_pair.0 == "key" {
            key = query_pair.1.to_string();
        }
    }

    // The API key in request doesn't match
    if key != API_KEY.lock().unwrap().as_str() {
        return bad_request("Not authorized");
    }

    let file_result = fs::read_to_string("lowestbin.json");
    if file_result.is_err() {
        return internal_error("Unable to open or read lowestbin.json");
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(file_result.unwrap()))
        .unwrap())
}

fn base() -> hyper::Result<Response<Body>> {
    // Returns information & statistics about the API
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            "{{
            \"success\":true,
            \"statistics\":
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
