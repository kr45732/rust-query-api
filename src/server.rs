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
use serde_json::json;
use surf::Url;
use tokio_postgres::Row;

use crate::config::{Config, Feature};
use crate::{statics::*, structs::*, utils::*};

/// Starts the server listening on URL
pub async fn start_server(config: Arc<Config>) {
    let server_address = config.full_url.parse().unwrap();
    let make_service = make_service_fn(|_| {
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
    info!("{} {}", req.method(), req.uri().path());

    if let (&Method::GET, "/") = (req.method(), req.uri().path()) {
        base(config).await
    } else if let (&Method::GET, "/query") = (req.method(), req.uri().path()) {
        if config.is_enabled(Feature::Query) {
            query(config, req).await
        } else {
            bad_request("Query feature is not enabled")
        }
    } else if let (&Method::GET, "/query_items") = (req.method(), req.uri().path()) {
        if config.is_enabled(Feature::Query) {
            query_items(config, req).await
        } else {
            bad_request("Query feature is not enabled")
        }
    } else if let (&Method::GET, "/pets") = (req.method(), req.uri().path()) {
        if config.is_enabled(Feature::Pets) {
            pets(config, req).await
        } else {
            bad_request("Pets feature is not enabled")
        }
    } else if let (&Method::GET, "/lowestbin") = (req.method(), req.uri().path()) {
        if config.is_enabled(Feature::Lowestbin) {
            lowestbin(config, req).await
        } else {
            bad_request("Lowest bins feature is not enabled")
        }
    } else if let (&Method::GET, "/underbin") = (req.method(), req.uri().path()) {
        if config.is_enabled(Feature::Underbin) {
            underbin(config, req).await
        } else {
            bad_request("Under bins feature is not enabled")
        }
    } else if let (&Method::GET, "/average_auction") = (req.method(), req.uri().path()) {
        if config.is_enabled(Feature::AverageAuction) {
            averages(config, req, vec!["average"]).await
        } else {
            bad_request("Average auction feature is not enabled")
        }
    } else if let (&Method::GET, "/average_bin") = (req.method(), req.uri().path()) {
        if config.is_enabled(Feature::AverageBin) {
            averages(config, req, vec!["average_bin"]).await
        } else {
            bad_request("Average bin feature is not enabled")
        }
    } else if let (&Method::GET, "/average") = (req.method(), req.uri().path()) {
        if config.is_enabled(Feature::AverageAuction) && config.is_enabled(Feature::AverageBin) {
            averages(config, req, vec!["average", "average_bin"]).await
        } else {
            bad_request("Both average auction and average bin feature are not enabled")
        }
    } else if let (&Method::GET, "/debug") = (req.method(), req.uri().path()) {
        if config.debug {
            debug_log(config, req).await
        } else {
            bad_request("Debug is not enabled")
        }
    } else if let (&Method::GET, "/info") = (req.method(), req.uri().path()) {
        if config.debug {
            info_log(config, req).await
        } else {
            bad_request("Debug is not enabled")
        }
    } else {
        not_found()
    }
}

/* /debug */
async fn debug_log(config: Arc<Config>, req: Request<Body>) -> hyper::Result<Response<Body>> {
    let mut key = String::new();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in Url::parse(&format!(
        "http://{}{}",
        config.full_url,
        &req.uri().to_string()
    ))
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
    let mut key = String::new();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in Url::parse(&format!(
        "http://{}{}",
        config.full_url,
        &req.uri().to_string()
    ))
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
    let mut query = String::new();
    let mut key = String::new();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in Url::parse(&format!(
        "http://{}{}",
        config.full_url,
        &req.uri().to_string()
    ))
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
    if !valid_api_key(config, key, false) {
        return bad_request("Not authorized");
    }

    if query.is_empty() {
        return bad_request("The query parameter cannot be empty");
    }

    let mut sql: String = String::from("SELECT * FROM pets WHERE name IN (");
    let mut param_vec: Vec<Box<String>> = Vec::new();
    let mut param_count = 1;

    let mut split = query.split(',');
    for pet_name in split.by_ref() {
        if param_count != 1 {
            sql.push(',');
        }
        sql.push_str(format!("${}", param_count).as_str());
        param_vec.push(Box::new(pet_name.to_string()));
        param_count += 1;
    }
    sql.push(')');

    let out: &Vec<&String> = &param_vec
        .iter()
        .map(std::ops::Deref::deref)
        .collect::<Vec<_>>();

    // Find and sort using query JSON
    let results_cursor = get_client().await.query_raw(&sql, out).await;

    if let Err(e) = results_cursor {
        return internal_error(&format!("Error when querying database: {}", e));
    }

    // Convert the cursor iterator to a vector
    let results_vec: Vec<PetsDatabaseItem> = results_cursor
        .unwrap()
        .try_collect::<Vec<Row>>()
        .await
        .unwrap()
        .into_iter()
        .map(PetsDatabaseItem::from)
        .collect();

    // Return the vector of auctions serialized into JSON
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&results_vec).unwrap()))
        .unwrap())
}

/* /average_auction or /average_bin or /average */
async fn averages(
    config: Arc<Config>,
    req: Request<Body>,
    tables: Vec<&str>,
) -> hyper::Result<Response<Body>> {
    let mut key = String::new();
    let mut time = 0;
    let mut step: usize = 1;

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in Url::parse(&format!(
        "http://{}{}",
        config.full_url,
        &req.uri().to_string()
    ))
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

    let old_method = config.old_method;

    // The API key in request doesn't match
    if !valid_api_key(config, key, false) {
        return bad_request("Not authorized");
    }

    if time < 0 {
        return bad_request("The time parameter cannot be negative");
    }

    // Map each item id to its prices and sales
    let avg_map: DashMap<String, AvgVec> = DashMap::new();

    for (idx, table) in tables.into_iter().enumerate() {
        // Find and sort using query JSON
        let results_cursor = get_client()
            .await
            .query(
                format!(
                    "SELECT time_t, prices FROM {} WHERE time_t > $1 ORDER BY time_t",
                    table
                )
                .as_str(),
                &[&time],
            )
            .await;

        if let Err(e) = results_cursor {
            return internal_error(&format!("Error when querying database: {}", e));
        }

        for row in results_cursor.unwrap() {
            let row_parsed = AverageDatabaseItem::from(row);
            for ele in row_parsed.prices {
                // If the id already exists in the map, append the new values, otherwise create a new entry
                if avg_map.contains_key(&ele.item_id) {
                    avg_map.alter(&ele.item_id.to_owned(), |_, value| {
                        value.update(ele, row_parsed.time_t, idx)
                    });
                } else {
                    avg_map.insert(
                        ele.item_id.to_owned(),
                        AvgVec::from(ele, row_parsed.time_t, idx),
                    );
                }
            }
        }
    }

    // Stores the values after averaging by 'step'
    let avg_map_final: DashMap<String, PartialAvgAh> = DashMap::new();
    for ele in avg_map {
        let mut count: i32 = 0;
        let mut sales: f32 = 0.0;
        let sales_arr = ele.1.get_sales();

        // Average the number of sales by the step parameter
        for i in (0..sales_arr.len()).step_by(step) {
            for j in i..(i + step) {
                if j >= sales_arr.len() {
                    break;
                }

                sales += sales_arr.get(j).unwrap();
            }
            count += 1;
        }

        avg_map_final.insert(
            ele.0,
            PartialAvgAh {
                price: ele.1.get_average(old_method),
                sales: sales / (count as f32),
            },
        );
    }

    // Return the vector of auctions or bins serialized into JSON
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_string(&avg_map_final).unwrap()))
        .unwrap())
}

/// HTTP Handler for query
async fn query(config: Arc<Config>, req: Request<Body>) -> hyper::Result<Response<Body>> {
    let mut query = String::new();
    let mut sort_by = String::new();
    let mut sort_order = String::new();
    let mut limit: i64 = 1;
    let mut key = String::new();
    let mut item_name = String::new();
    let mut tier = String::new();
    let mut item_id = String::new();
    let mut internal_id = String::new();
    let mut enchants = String::new();
    let mut end: i64 = -1;
    let mut bids = String::new();
    let mut bin = Option::None;
    let mut potato_books = -1;
    let mut stars = -1;
    let mut farming_for_dummies = -1;
    let mut transmission_tuner = -1;
    let mut mana_disintegrator = -1;
    let mut reforge = String::new();
    let mut rune = String::new();
    let mut skin = String::new();
    let mut power_scroll = String::new();
    let mut drill_upgrade_module = String::new();
    let mut drill_fuel_tank = String::new();
    let mut drill_engine = String::new();
    let mut dye = String::new();
    let mut accessory_enrichment = String::new();
    let mut recombobulated = Option::None;
    let mut wood_singularity = Option::None;
    let mut art_of_war = Option::None;
    let mut art_of_peace = Option::None;
    let mut etherwarp = Option::None;
    let mut necron_scrolls = String::new();
    let mut gemstones = String::new();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in Url::parse(&format!(
        "http://{}{}",
        config.full_url,
        &req.uri().to_string()
    ))
    .unwrap()
    .query_pairs()
    {
        match query_pair.0.to_string().as_str() {
            "query" => query = query_pair.1.to_string(),
            "sort_by" => sort_by = query_pair.1.to_string(),
            "sort_order" => sort_order = query_pair.1.to_string(),
            "limit" => match query_pair.1.to_string().parse::<i64>() {
                Ok(limit_int) => limit = limit_int,
                Err(e) => return bad_request(&format!("Error parsing limit parameter: {}", e)),
            },
            "key" => key = query_pair.1.to_string(),
            "item_name" => item_name = query_pair.1.to_string(),
            "tier" => tier = query_pair.1.to_string(),
            "item_id" => item_id = query_pair.1.to_string(),
            "internal_id" => internal_id = query_pair.1.to_string(),
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
            "potato_books" => match query_pair.1.to_string().parse::<i16>() {
                Ok(potato_books_int) => potato_books = potato_books_int,
                Err(e) => {
                    return bad_request(&format!("Error parsing potato_books parameter: {}", e))
                }
            },
            "stars" => match query_pair.1.to_string().parse::<i16>() {
                Ok(stars_int) => stars = stars_int,
                Err(e) => return bad_request(&format!("Error parsing stars parameter: {}", e)),
            },
            "farming_for_dummies" => match query_pair.1.to_string().parse::<i16>() {
                Ok(farming_for_dummies_int) => farming_for_dummies = farming_for_dummies_int,
                Err(e) => {
                    return bad_request(&format!(
                        "Error parsing farming_for_dummies parameter: {}",
                        e
                    ))
                }
            },
            "transmission_tuner" => match query_pair.1.to_string().parse::<i16>() {
                Ok(transmission_tuner_int) => transmission_tuner = transmission_tuner_int,
                Err(e) => {
                    return bad_request(&format!(
                        "Error parsing transmission_tuner parameter: {}",
                        e
                    ))
                }
            },
            "mana_disintegrator" => match query_pair.1.to_string().parse::<i16>() {
                Ok(mana_disintegrator_int) => mana_disintegrator = mana_disintegrator_int,
                Err(e) => {
                    return bad_request(&format!(
                        "Error parsing mana_disintegrator parameter: {}",
                        e
                    ))
                }
            },
            "reforge" => reforge = query_pair.1.to_string(),
            "rune" => rune = query_pair.1.to_string(),
            "skin" => skin = query_pair.1.to_string(),
            "power_scroll" => power_scroll = query_pair.1.to_string(),
            "drill_upgrade_module" => drill_upgrade_module = query_pair.1.to_string(),
            "drill_fuel_tank" => drill_fuel_tank = query_pair.1.to_string(),
            "drill_engine" => drill_engine = query_pair.1.to_string(),
            "dye" => dye = query_pair.1.to_string(),
            "accessory_enrichment" => accessory_enrichment = query_pair.1.to_string(),
            "recombobulated" => match query_pair.1.to_string().parse::<bool>() {
                Ok(recombobulated_bool) => recombobulated = Some(recombobulated_bool),
                Err(e) => {
                    return bad_request(&format!("Error parsing recombobulated parameter: {}", e))
                }
            },
            "wood_singularity" => match query_pair.1.to_string().parse::<bool>() {
                Ok(wood_singularity_bool) => wood_singularity = Some(wood_singularity_bool),
                Err(e) => {
                    return bad_request(&format!("Error parsing wood_singularity parameter: {}", e))
                }
            },
            "art_of_war" => match query_pair.1.to_string().parse::<bool>() {
                Ok(art_of_war_bool) => art_of_war = Some(art_of_war_bool),
                Err(e) => {
                    return bad_request(&format!("Error parsing art_of_war parameter: {}", e))
                }
            },
            "art_of_peace" => match query_pair.1.to_string().parse::<bool>() {
                Ok(art_of_peace_bool) => art_of_peace = Some(art_of_peace_bool),
                Err(e) => {
                    return bad_request(&format!("Error parsing art_of_peace parameter: {}", e))
                }
            },
            "etherwarp" => match query_pair.1.to_string().parse::<bool>() {
                Ok(etherwarp_bool) => etherwarp = Some(etherwarp_bool),
                Err(e) => return bad_request(&format!("Error parsing etherwarp parameter: {}", e)),
            },
            "necron_scrolls" => necron_scrolls = query_pair.1.to_string(),
            "gemstones" => gemstones = query_pair.1.to_string(),
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

        let mut is_sorting = false;

        if !bids.is_empty() {
            sql = String::from("SELECT * FROM query, unnest(bids) AS bid WHERE bid.bidder = $1");
            param_vec.push(&bids);
            param_count += 1;
        } else {
            sql = String::from("SELECT * FROM query WHERE");
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
        if !internal_id.is_empty() {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" internal_id = ${}", param_count).as_str());
            param_vec.push(&internal_id);
            param_count += 1;
        }
        let enchants_split: Vec<String>;
        if !enchants.is_empty() {
            if param_count != 1 {
                sql.push_str(" AND");
            }

            sql.push_str(" enchants @> ARRAY[");
            let start_param_count = param_count;
            enchants_split = enchants.split(",").map(|s| s.to_string()).collect();
            for enchant in enchants_split.iter() {
                if param_count != start_param_count {
                    sql.push(',');
                }

                sql.push_str(format!("${}", param_count).as_str());

                param_vec.push(enchant);
                param_count += 1;
            }
            sql.push_str("]");
        }
        if end >= 0 {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" end_t > ${}", param_count).as_str());
            param_vec.push(&end);
            param_count += 1;
        }
        if potato_books >= 0 {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" potato_books = ${}", param_count).as_str());
            param_vec.push(&potato_books);
            param_count += 1;
        }
        if stars >= 0 {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" stars = ${}", param_count).as_str());
            param_vec.push(&stars);
            param_count += 1;
        }
        if farming_for_dummies >= 0 {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" farming_for_dummies > ${}", param_count).as_str());
            param_vec.push(&farming_for_dummies);
            param_count += 1;
        }
        if transmission_tuner >= 0 {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" transmission_tuner > ${}", param_count).as_str());
            param_vec.push(&transmission_tuner);
            param_count += 1;
        }
        if mana_disintegrator >= 0 {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" mana_disintegrator > ${}", param_count).as_str());
            param_vec.push(&mana_disintegrator);
            param_count += 1;
        }
        if !reforge.is_empty() {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" reforge = ${}", param_count).as_str());
            param_vec.push(&reforge);
            param_count += 1;
        }
        if !rune.is_empty() {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" rune = ${}", param_count).as_str());
            param_vec.push(&rune);
            param_count += 1;
        }
        if !skin.is_empty() {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" skin = ${}", param_count).as_str());
            param_vec.push(&skin);
            param_count += 1;
        }
        if !power_scroll.is_empty() {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" power_scroll = ${}", param_count).as_str());
            param_vec.push(&power_scroll);
            param_count += 1;
        }
        if !drill_upgrade_module.is_empty() {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" drill_upgrade_module = ${}", param_count).as_str());
            param_vec.push(&drill_upgrade_module);
            param_count += 1;
        }
        if !drill_fuel_tank.is_empty() {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" drill_fuel_tank = ${}", param_count).as_str());
            param_vec.push(&drill_fuel_tank);
            param_count += 1;
        }
        if !drill_engine.is_empty() {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" drill_engine = ${}", param_count).as_str());
            param_vec.push(&drill_engine);
            param_count += 1;
        }
        if !dye.is_empty() {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" dye = ${}", param_count).as_str());
            param_vec.push(&dye);
            param_count += 1;
        }
        if !accessory_enrichment.is_empty() {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" accessory_enrichment = ${}", param_count).as_str());
            param_vec.push(&accessory_enrichment);
            param_count += 1;
        }
        let recombobulated_unwrapped;
        if recombobulated.is_some() {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" recombobulated = ${}", param_count).as_str());
            recombobulated_unwrapped = recombobulated.unwrap();
            param_vec.push(&recombobulated_unwrapped);
            param_count += 1;
        }
        let wood_singularity_unwrapped;
        if wood_singularity.is_some() {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" wood_singularity = ${}", param_count).as_str());
            wood_singularity_unwrapped = wood_singularity.unwrap();
            param_vec.push(&wood_singularity_unwrapped);
            param_count += 1;
        }
        let art_of_war_unwrapped;
        if art_of_war.is_some() {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" art_of_war = ${}", param_count).as_str());
            art_of_war_unwrapped = art_of_war.unwrap();
            param_vec.push(&art_of_war_unwrapped);
            param_count += 1;
        }
        let art_of_peace_unwrapped;
        if art_of_peace.is_some() {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" art_of_peace = ${}", param_count).as_str());
            art_of_peace_unwrapped = art_of_peace.unwrap();
            param_vec.push(&art_of_peace_unwrapped);
            param_count += 1;
        }
        let etherwarp_unwrapped;
        if etherwarp.is_some() {
            if param_count != 1 {
                sql.push_str(" AND");
            }
            sql.push_str(format!(" etherwarp = ${}", param_count).as_str());
            etherwarp_unwrapped = etherwarp.unwrap();
            param_vec.push(&etherwarp_unwrapped);
            param_count += 1;
        }
        let necron_scrolls_split: Vec<String>;
        if !necron_scrolls.is_empty() {
            if param_count != 1 {
                sql.push_str(" AND");
            }

            sql.push_str(" necron_scrolls @> ARRAY[");
            let start_param_count = param_count;
            necron_scrolls_split = necron_scrolls.split(",").map(|s| s.to_string()).collect();
            for necron_scroll in necron_scrolls_split.iter() {
                if param_count != start_param_count {
                    sql.push(',');
                }

                sql.push_str(format!("${}", param_count).as_str());

                param_vec.push(necron_scroll);
                param_count += 1;
            }
            sql.push_str("]");
        }
        let gemstones_split: Vec<String>;
        if !gemstones.is_empty() {
            if param_count != 1 {
                sql.push_str(" AND");
            }

            sql.push_str(" gemstones @> ARRAY[");
            let start_param_count = param_count;
            gemstones_split = gemstones.split(",").map(|s| s.to_string()).collect();
            for gemstone in gemstones_split.iter() {
                if param_count != start_param_count {
                    sql.push(',');
                }

                sql.push_str(format!("${}", param_count).as_str());

                param_vec.push(gemstone);
                param_count += 1;
            }
            sql.push_str("]");
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
        if (sort_by == "starting_bid" || sort_by == "highest_bid")
            && (sort_order == "ASC" || sort_order == "DESC")
        {
            if param_count == 1 {
                sql.push_str(" 1=1"); // Handles unfinished WHERE
            }
            sql.push_str(format!(" ORDER BY {} {}", sort_by, sort_order).as_str());
            is_sorting = true;
        };

        // Prevent fetching too many rows
        if (limit <= 0 || limit >= 500) && !valid_api_key(config.clone(), key.to_owned(), true) {
            return bad_request("Not authorized");
        }
        if param_count == 1 && !is_sorting {
            sql.push_str(" 1=1"); // Handles unfinished WHERE
        }
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
    let results_vec = results_cursor
        .unwrap()
        .into_iter()
        .map(QueryDatabaseItem::from)
        .collect::<Vec<QueryDatabaseItem>>();

    // Return the vector of auctions serialized into JSON
    Ok(Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(serde_json::to_vec(&results_vec).unwrap()))
        .unwrap())
}

/* /query_items */
async fn query_items(config: Arc<Config>, req: Request<Body>) -> hyper::Result<Response<Body>> {
    let mut key = String::new();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in Url::parse(&format!(
        "http://{}{}",
        config.full_url,
        &req.uri().to_string()
    ))
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
    let mut key = String::new();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in Url::parse(&format!("http://{}{}", config.full_url, &req.uri()))
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
    let mut key = String::new();

    // Reads the query parameters from the request and stores them in the corresponding variable
    for query_pair in Url::parse(&format!("http://{}{}", config.full_url, &req.uri()))
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
        .body(Body::from(
            json!({
                "success":true,
                "enabled_features": {
                    "query":config.is_enabled(Feature::Query),
                    "pets":config.is_enabled(Feature::Pets),
                    "lowestbin":config.is_enabled(Feature::Lowestbin),
                    "underbin": config.is_enabled(Feature::Underbin),
                    "average_auction":config.is_enabled(Feature::AverageAuction),
                    "average_bin":config.is_enabled(Feature::AverageBin),
                },
                "statistics": {
                    "is_updating":*IS_UPDATING.lock().await,
                    "total_updates":*TOTAL_UPDATES.lock().await,
                    "last_updated":*LAST_UPDATED.lock().await
                }
            })
            .to_string(),
        ))
        .unwrap())
}

fn http_err(status: StatusCode, reason: &str) -> hyper::Result<Response<Body>> {
    Ok(Response::builder()
        .status(status)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(
            json!({"success": false, "reason": reason}).to_string(),
        ))
        .unwrap())
}

fn bad_request(reason: &str) -> hyper::Result<Response<Body>> {
    http_err(StatusCode::BAD_REQUEST, reason)
}

fn not_found() -> hyper::Result<Response<Body>> {
    http_err(StatusCode::NOT_FOUND, "Not found")
}

fn internal_error(reason: &str) -> hyper::Result<Response<Body>> {
    http_err(StatusCode::INTERNAL_SERVER_ERROR, reason)
}
