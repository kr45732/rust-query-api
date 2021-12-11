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

use crate::{statics::*, structs::*, utils::*};
use dashmap::DashMap;
use hyper::{
    header,
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use log::info;
use postgres_types::ToSql;
use reqwest::Url;
use std::fs;
use substring::Substring;

/* Starts the server listening on URL */
pub async fn start_server() {
    let server_address = URL.lock().unwrap().parse().unwrap();

    let make_service =
        make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(handle_response)) });

    let server = Server::bind(&server_address).serve(make_service);

    println!("Listening on http://{}", server_address);
    info(format!("Listening on http://{}", server_address)).await;

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
    } else if let (&Method::GET, "/underbin") = (req.method(), req.uri().path()) {
        if *ENABLE_UNDERBIN.lock().unwrap() {
            underbin(req).await
        } else {
            bad_request("Under bins feature is not enabled")
        }
    } else if let (&Method::GET, "/average_auction") = (req.method(), req.uri().path()) {
        if *ENABLE_AVERAGE_AUCTION.lock().unwrap() {
            averag_auction(req).await
        } else {
            bad_request("Average auction feature is not enabled")
        }
    } else {
        not_found()
    }
}

/* /pets */
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
    if !valid_api_key(key, true) {
        return bad_request("Not authorized");
    }

    if query.len() == 0 {
        return bad_request("The query parameter cannot be empty");
    }

    unsafe {
        let database_ref = DATABASE.as_ref();

        // Check to see if the database is connected
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

/* /average_auction */
async fn averag_auction(req: Request<Body>) -> hyper::Result<Response<Body>> {
    let mut query = -1;
    let mut key = "".to_string();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in
        Url::parse(&format!("http://{}{}", URL.lock().unwrap(), &req.uri().to_string()).to_string())
            .unwrap()
            .query_pairs()
    {
        if query_pair.0 == "query" {
            match query_pair.1.to_string().parse::<i64>() {
                Ok(query_int) => query = query_int,
                Err(e) => return bad_request(&format!("Error parsing query parameter: {}", e)),
            }
        } else if query_pair.0 == "key" {
            key = query_pair.1.to_string();
        }
    }

    // The API key in request doesn't match
    if !valid_api_key(key, false) {
        return bad_request("Not authorized");
    }

    if query <= 0 {
        return bad_request("The query parameter must be provided and positive");
    }

    unsafe {
        let database_ref = DATABASE.as_ref();

        // Check to see if the database is connected
        if database_ref.is_none() {
            return internal_error("Database isn't connected");
        }

        let results_cursor;
        // Find and sort using query JSON
        results_cursor = database_ref
            .unwrap()
            .query("SELECT * FROM average WHERE time_t > $1", &[&query])
            .await;

        if let Err(e) = results_cursor {
            return internal_error(&format!("Error when querying database: {}", e).to_string());
        }

        let avg_ah_map: DashMap<String, AvgAhSum> = DashMap::new();
        results_cursor.unwrap().into_iter().for_each(|ele_row| {
            let ele_db = AverageDatabaseItem::from(ele_row);
            for ele in ele_db.prices {
                if avg_ah_map.contains_key(&ele.item_id) {
                    avg_ah_map.alter(&ele.item_id, |_, mut value| {
                        value.add(ele.amount);
                        return value;
                    });
                } else {
                    avg_ah_map.insert(
                        ele.item_id,
                        AvgAhSum {
                            sum: ele.amount,
                            count: 1,
                        },
                    );
                }
            }
        });

        let mut avg_ah_prices: Vec<AvgAh> = Vec::new();
        for ele in avg_ah_map {
            avg_ah_prices.push(AvgAh {
                item_id: ele.0,
                amount: ele.1.get_average(),
            })
        }

        // Return the vector of auctions serialized into JSON
        Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(serde_json::to_vec(&avg_ah_prices).unwrap()))
            .unwrap())
    }
}

/* /query */
async fn query(req: Request<Body>) -> hyper::Result<Response<Body>> {
    let mut query = "".to_string();
    let mut sort = "".to_string();
    let mut limit: i64 = 1;
    let mut key = "".to_string();
    let mut item_name = "".to_string();
    let mut tier = "".to_string();
    let mut item_id = "".to_string();
    let mut enchants = "".to_string();
    let mut end: i64 = -1;
    let mut bids = "".to_string();
    let mut bin = Option::None;

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in
        Url::parse(&format!("http://{}{}", URL.lock().unwrap(), &req.uri().to_string()).to_string())
            .unwrap()
            .query_pairs()
    {
        match query_pair.0.to_string().as_str() {
            "query" => query = query_pair.1.to_string(),
            "sort" => sort = query_pair.1.to_string(),
            "limit" => match query_pair.1.to_string().parse::<i64>() {
                Ok(limit_int) => limit = limit_int,
                Err(e) => return bad_request(&format!("Error parsing limit parameter: {}", e)),
            },
            "key" => key = query_pair.1.to_string(),
            "item_name" => item_name = query_pair.1.to_string(),
            "tier" => tier = query_pair.1.to_string(),
            "item_id" => item_id = query_pair.1.to_string(),
            "enchants" => enchants = query_pair.1.to_string(),
            "end" => match query_pair.1.to_string().parse::<i64>() {
                Ok(end_int) => end = end_int,
                Err(e) => return bad_request(&format!("Error parsing end parameter: {}", e)),
            },
            "bids" => bids = query_pair.1.to_string(),
            "bin" => match query_pair.1.to_string().parse::<bool>() {
                Ok(bin_bool) => bin = Some(bin_bool),
                Err(e) => return bad_request(&format!("Error parsing bin parameter: {}", e)),
            },
            _ => {}
        }
    }

    if !valid_api_key(key.to_owned(), false) {
        return bad_request("Not authorized");
    }

    unsafe {
        // Checks if the database is connected
        if DATABASE.as_ref().is_none() {
            return internal_error("Database isn't connected");
        }

        let database_ref = DATABASE.as_ref().unwrap();
        let results_cursor;

        // Find and sort using query
        if query.is_empty() {
            let mut sql;
            let mut param_vec: Vec<&(dyn ToSql + Sync)> = Vec::new();
            let mut param_count = 1;

            if !bids.is_empty() {
                sql = "SELECT * FROM query, unnest(bids) AS bid WHERE bid.bidder = $1".to_string();
                param_vec.push(&bids);
                param_count += 1;
            } else {
                sql = "SELECT * FROM query WHERE".to_string();
            }

            if !tier.is_empty() {
                if param_count != 1 {
                    sql.push_str(" AND");
                }
                sql.push_str(format!(" tier = ${}", param_count).as_str());
                param_vec.push(&tier);
                param_count += 1;
            }
            if !item_name.is_empty() {
                if param_count != 1 {
                    sql.push_str(" AND");
                }
                sql.push_str(format!(" item_name ILIKE ${}", param_count).as_str());
                param_vec.push(&item_name);
                param_count += 1;
            }
            if !item_id.is_empty() {
                if param_count != 1 {
                    sql.push_str(" AND");
                }
                sql.push_str(format!(" item_id = ${}", param_count).as_str());
                param_vec.push(&item_id);
                param_count += 1;
            }
            if !enchants.is_empty() {
                if param_count != 1 {
                    sql.push_str(" AND");
                }
                sql.push_str(format!(" ${} = ANY (enchants)", param_count).as_str());
                param_vec.push(&enchants);
                param_count += 1;
            };
            if end >= 0 {
                if param_count != 1 {
                    sql.push_str(" AND");
                }
                sql.push_str(format!(" end_t > ${}", param_count).as_str());
                param_vec.push(&end);
                param_count += 1;
            }
            let bin_unwrapped;
            if bin.is_some() {
                if param_count != 1 {
                    sql.push_str(" AND");
                }
                sql.push_str(format!(" bin = ${}", param_count).as_str());
                bin_unwrapped = bin.unwrap();
                param_vec.push(&bin_unwrapped);
                param_count += 1;
            }
            if !sort.is_empty() {
                if sort == "ASC" {
                    sql.push_str(" ORDER BY starting_bid ASC");
                } else if sort == "DESC" {
                    sql.push_str(" ORDER BY starting_bid DESC");
                }
            };
            if limit > 0 {
                sql.push_str(format!(" LIMIT ${}", param_count).as_str());
                param_vec.push(&limit);
            }

            results_cursor = database_ref.query(&sql, &param_vec).await;
        } else {
            if !valid_api_key(key, true) {
                return bad_request("Not authorized");
            }

            results_cursor = database_ref
                .query(&format!("SELECT * FROM query WHERE {}", query), &[])
                .await;
        }

        if let Err(e) = results_cursor {
            return internal_error(&format!("Error when querying database: {}", e));
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

/* /lowestbin */
async fn lowestbin(req: Request<Body>) -> hyper::Result<Response<Body>> {
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

    if !valid_api_key(key, false) {
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

/* /underbin */
async fn underbin(req: Request<Body>) -> hyper::Result<Response<Body>> {
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

    if !valid_api_key(key, false) {
        return bad_request("Not authorized");
    }

    let file_result = fs::read_to_string("underbin.json");
    if file_result.is_err() {
        return internal_error("Unable to open or read underbin.json");
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(file_result.unwrap()))
        .unwrap())
}

/* / */
fn base() -> hyper::Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            "{{
            \"success\":true,
            \"enabled_features\":{{
                \"QUERY\":{},
                \"PETS\":{},
                \"LOWESTBIN\":{},
                \"UNDERBIN\":{},
                \"AVERAGE_AUCTION\":{}
            }},\"statistics\":
            {{
                \"is_updating\":{},
                \"total_updates\":{},
                \"last_updated\":{}
            }}
        }}",
            *ENABLE_QUERY.lock().unwrap(),
            *ENABLE_PETS.lock().unwrap(),
            *ENABLE_LOWESTBIN.lock().unwrap(),
            *ENABLE_UNDERBIN.lock().unwrap(),
            *ENABLE_AVERAGE_AUCTION.lock().unwrap(),
            *IS_UPDATING.lock().unwrap(),
            *TOTAL_UPDATES.lock().unwrap(),
            *LAST_UPDATED.lock().unwrap()
        )))
        .unwrap())
}

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
