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

use std::fs;
use std::sync::Arc;

use dashmap::DashMap;
use futures::TryStreamExt;
use hyper::{
    header,
    service::{make_service_fn, service_fn},
    Body, Method, Request, Response, Server, StatusCode,
};
use log::info;
use postgres_types::ToSql;
use substring::Substring;
use surf::Url;
use tokio_postgres::Row;

use crate::config::{Config, Feature};
use crate::{statics::*, structs::*, utils::*};

/// Starts the server listening on URL
pub async fn start_server(config: Arc<Config>) {
    let server_address = config.full_url.parse().unwrap();
    let make_service = make_service_fn(move |_| {
        let captured_config = config.clone();
        async {
            Ok::<_, hyper::Error>(service_fn(move |req| {
                handle_response(captured_config.clone(), req)
            }))
        }
    });

    let server = Server::bind(&server_address).serve(make_service);

    info(format!("Listening on http://{}", server_address));
    if let Err(e) = server.await {
        error(format!("Error when starting server: {}", e));
    }
}

/* Handles http requests to the server */
async fn handle_response(config: Arc<Config>, req: Request<Body>) -> hyper::Result<Response<Body>> {
    info!("{} {}", req.method(), req.uri().path().substring(0, 30));

    if let (&Method::GET, "/") = (req.method(), req.uri().path()) {
        base(config).await
    } else if let (&Method::GET, "/query") = (req.method(), req.uri().path()) {
        if config.enabled_features.contains(Feature::QUERY) {
            query(config, req).await
        } else {
            bad_request("Query feature is not enabled")
        }
    } else if let (&Method::GET, "/query_items") = (req.method(), req.uri().path()) {
        if config.enabled_features.contains(Feature::QUERY) {
            query_items(config, req).await
        } else {
            bad_request("Query feature is not enabled")
        }
    } else if let (&Method::GET, "/pets") = (req.method(), req.uri().path()) {
        if config.enabled_features.contains(Feature::PETS) {
            pets(config, req).await
        } else {
            bad_request("Pets feature is not enabled")
        }
    } else if let (&Method::GET, "/lowestbin") = (req.method(), req.uri().path()) {
        if config.enabled_features.contains(Feature::LOWESTBIN) {
            lowestbin(config, req).await
        } else {
            bad_request("Lowest bins feature is not enabled")
        }
    } else if let (&Method::GET, "/underbin") = (req.method(), req.uri().path()) {
        if config.enabled_features.contains(Feature::UNDERBIN) {
            underbin(config, req).await
        } else {
            bad_request("Under bins feature is not enabled")
        }
    } else if let (&Method::GET, "/average_auction") = (req.method(), req.uri().path()) {
        if config.enabled_features.contains(Feature::AVERAGE_AUCTION) {
            average_auction(config, req).await
        } else {
            bad_request("Average auction feature is not enabled")
        }
    } else if let (&Method::GET, "/debug") = (req.method(), req.uri().path()) {
        debug_log(config, req).await
    } else if let (&Method::GET, "/info") = (req.method(), req.uri().path()) {
        info_log(config, req).await
    } else {
        not_found()
    }
}

/* /debug */
async fn debug_log(config: Arc<Config>, req: Request<Body>) -> hyper::Result<Response<Body>> {
    let mut key = "".to_string();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in
        Url::parse(&format!("http://{}{}", config.full_url, &req.uri().to_string()).to_string())
            .unwrap()
            .query_pairs()
    {
        if query_pair.0 == "key" {
            key = query_pair.1.to_string();
        }
    }

    if !valid_api_key(config, key, true) {
        return bad_request("Not authorized");
    }

    let file_result = fs::read_to_string("debug.log");
    if file_result.is_err() {
        return internal_error("Unable to open or read debug.log");
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(file_result.unwrap()))
        .unwrap())
}

/* /info */
async fn info_log(config: Arc<Config>, req: Request<Body>) -> hyper::Result<Response<Body>> {
    let mut key = "".to_string();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in
        Url::parse(&format!("http://{}{}", config.full_url, &req.uri().to_string()).to_string())
            .unwrap()
            .query_pairs()
    {
        if query_pair.0 == "key" {
            key = query_pair.1.to_string();
        }
    }

    if !valid_api_key(config, key, true) {
        return bad_request("Not authorized");
    }

    let file_result = fs::read_to_string("info.log");
    if file_result.is_err() {
        return internal_error("Unable to open or read info.log");
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .body(Body::from(file_result.unwrap()))
        .unwrap())
}

/* /pets */
async fn pets(config: Arc<Config>, req: Request<Body>) -> hyper::Result<Response<Body>> {
    let mut query = "".to_string();
    let mut key = "".to_string();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in
        Url::parse(&format!("http://{}{}", config.full_url, &req.uri().to_string()).to_string())
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
    if !valid_api_key(config, key, true) {
        return bad_request("Not authorized");
    }

    if query.len() == 0 {
        return bad_request("The query parameter cannot be empty");
    }

    let database_ref = get_client().await;

    let mut sql: String = "SELECT * FROM pets WHERE name IN (".to_string();
    let mut param_vec: Vec<Box<String>> = Vec::new();
    let mut param_count = 1;

    let mut split = query.split(",");
    while let Some(pet_name) = split.next() {
        if param_count != 1 {
            sql.push_str(",");
        }
        sql.push_str(format!("${}", param_count).as_str());
        param_vec.push(Box::new(pet_name.to_string()));
        param_count += 1;
    }
    sql.push_str(")");

    let out: &Vec<&String> = &param_vec
        .iter()
        .map(std::ops::Deref::deref)
        .collect::<Vec<_>>();

    // Find and sort using query JSON
    let results_cursor = database_ref.query_raw(&sql, out).await;

    if let Err(e) = results_cursor {
        return internal_error(&format!("Error when querying database: {}", e).to_string());
    }

    // Convert the cursor iterator to a vector
    let results_vec: Vec<PetsDatabaseItem> = results_cursor
        .unwrap()
        .try_collect::<Vec<Row>>()
        .await
        .unwrap()
        .into_iter()
        .map(|ele| PetsDatabaseItem::from(ele))
        .collect();

    // Return the vector of auctions serialized into JSON
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&results_vec).unwrap()))
        .unwrap())
}

/* /average_auction */
async fn average_auction(config: Arc<Config>, req: Request<Body>) -> hyper::Result<Response<Body>> {
    let mut key = "".to_string();
    let mut time = -1;
    let mut step: usize = 1;

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in
        Url::parse(&format!("http://{}{}", config.full_url, &req.uri().to_string()).to_string())
            .unwrap()
            .query_pairs()
    {
        match query_pair.0.to_string().as_str() {
            "time" => match query_pair.1.to_string().parse::<i64>() {
                Ok(time_int) => time = time_int,
                Err(e) => return bad_request(&format!("Error parsing time parameter: {}", e)),
            },
            "step" => match query_pair.1.to_string().parse::<usize>() {
                Ok(step_int) => step = step_int,
                Err(e) => return bad_request(&format!("Error parsing step parameter: {}", e)),
            },
            "key" => key = query_pair.1.to_string(),
            _ => {}
        }
    }

    // The API key in request doesn't match
    if !valid_api_key(config, key, false) {
        return bad_request("Not authorized");
    }

    if time <= 0 {
        return bad_request("The time parameter must be provided and positive");
    }

    let database_ref = get_client().await;

    // Find and sort using query JSON
    let results_cursor = database_ref
        .query(
            "SELECT * FROM average WHERE time_t > $1 ORDER BY time_t",
            &[&time],
        )
        .await;

    if let Err(e) = results_cursor {
        return internal_error(&format!("Error when querying database: {}", e).to_string());
    }

    // Map each item id to its prices and sales
    let avg_ah_map: DashMap<String, AvgAhVec> = DashMap::new();
    results_cursor.unwrap().into_iter().for_each(|ele_row| {
        for ele in AverageDatabaseItem::from(ele_row).prices {
            // If the id already exists in the map, append the new values, otherwise create a new entry
            if avg_ah_map.contains_key(&ele.item_id) {
                avg_ah_map.alter(&ele.item_id, |_, value| value.add(&ele));
            } else {
                avg_ah_map.insert(ele.item_id.to_owned(), AvgAhVec::from(&ele));
            }
        }
    });

    // Stores the values after averaging by 'step'
    let avg_ah_map_final: DashMap<String, AvgAh> = DashMap::new();
    for ele in avg_ah_map {
        let mut count: i64 = 0;
        let mut sales: f32 = 0.0;

        // Average the number of sales by the step parameter
        for i in (0..ele.1.sales.len()).step_by(step) {
            for j in i..(i + step) {
                if j >= ele.1.sales.len() {
                    break;
                }

                sales += ele.1.sales.get(j).unwrap();
            }
            count += 1;
        }

        avg_ah_map_final.insert(
            ele.0.to_owned(),
            AvgAh {
                item_id: ele.0,
                price: ele.1.get_average(),
                sales: sales / (count as f32),
            },
        );
    }

    // Return the vector of auctions serialized into JSON
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            serde_json::to_string(&avg_ah_map_final).unwrap(),
        ))
        .unwrap())
}

/// HTTP Handler for query
async fn query(config: Arc<Config>, req: Request<Body>) -> hyper::Result<Response<Body>> {
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
        Url::parse(&format!("http://{}{}", config.full_url, &req.uri().to_string()).to_string())
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

    if !valid_api_key(config.clone(), key.to_owned(), false) {
        return bad_request("Not authorized");
    }

    let database_ref = get_client().await;

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
        if !valid_api_key(config, key, true) {
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

/* /query_items */
async fn query_items(config: Arc<Config>, req: Request<Body>) -> hyper::Result<Response<Body>> {
    let mut key = "".to_string();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in
        Url::parse(&format!("http://{}{}", config.full_url, &req.uri().to_string()).to_string())
            .unwrap()
            .query_pairs()
    {
        if query_pair.0 == "key" {
            key = query_pair.1.to_string();
        }
    }

    if !valid_api_key(config, key, false) {
        return bad_request("Not authorized");
    }

    let file_result = fs::read_to_string("query_items.json");
    if file_result.is_err() {
        return internal_error("Unable to open or read query_items.json");
    }

    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(file_result.unwrap()))
        .unwrap())
}

/* /lowestbin */
async fn lowestbin(config: Arc<Config>, req: Request<Body>) -> hyper::Result<Response<Body>> {
    let mut key = "".to_string();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in
        Url::parse(&format!("http://{}{}", config.full_url, &req.uri().to_string()).to_string())
            .unwrap()
            .query_pairs()
    {
        if query_pair.0 == "key" {
            key = query_pair.1.to_string();
        }
    }

    if !valid_api_key(config, key, false) {
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
async fn underbin(config: Arc<Config>, req: Request<Body>) -> hyper::Result<Response<Body>> {
    let mut key = "".to_string();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in
        Url::parse(&format!("http://{}{}", config.full_url, &req.uri().to_string()).to_string())
            .unwrap()
            .query_pairs()
    {
        if query_pair.0 == "key" {
            key = query_pair.1.to_string();
        }
    }

    if !valid_api_key(config, key, false) {
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
async fn base(config: Arc<Config>) -> hyper::Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            "{{
            \"success\":true,
            \"enabled_features\":{{
                \"query\":{},
                \"pets\":{},
                \"lowestbin\":{},
                \"underbin\":{},
                \"average_auction\":{}
            }},\"statistics\":
            {{
                \"is_updating\":{},
                \"total_updates\":{},
                \"last_updated\":{}
            }}
        }}",
            config.enabled_features.contains(Feature::QUERY),
            config.enabled_features.contains(Feature::PETS),
            config.enabled_features.contains(Feature::LOWESTBIN),
            config.enabled_features.contains(Feature::UNDERBIN),
            config.enabled_features.contains(Feature::AVERAGE_AUCTION),
            *IS_UPDATING.lock().await,
            *TOTAL_UPDATES.lock().await,
            *LAST_UPDATED.lock().await
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
