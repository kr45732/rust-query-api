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

use std::sync::Arc;
use std::{fs, time::Instant};

use chrono::Utc;
use dashmap::{DashMap, DashSet};
use futures::{stream::FuturesUnordered, StreamExt};
use log::{debug, info};
use serde_json::{json, Value};

use crate::config::{Config, Feature};
use crate::{statics::*, structs::*, utils::*};

/// Update the enabled APIs
pub async fn update_auctions(config: Arc<Config>) {
    info("Fetching auctions...".to_string());

    let started = Instant::now();
    let started_epoch = Utc::now().timestamp_millis();
    *IS_UPDATING.lock().await = true;

    // Stores all auction uuids in auctions vector to prevent duplicates
    let mut inserted_uuids: DashSet<String> = DashSet::new();
    // Stores query prices
    let mut query_prices: Vec<DatabaseItem> = Vec::new();
    // Stores pet prices
    let mut pet_prices: DashMap<String, i64> = DashMap::new();
    // Stores bin prices
    let mut bin_prices: DashMap<String, i64> = DashMap::new();
    // Stores under bin prices
    let mut under_bin_prices: Vec<Value> = Vec::new();
    // Stores average auction prices
    let mut avg_ah_prices: Vec<AvgAh> = Vec::new();
    // Previous bin prices
    let past_bin_prices: DashMap<String, i64> = serde_json::from_str(
        &fs::read_to_string("lowestbin.json").unwrap_or_else(|_| "{}".to_string()),
    )
    .unwrap();

    // Get which APIs to update
    let update_query = config.enabled_features.contains(Feature::Query);
    let update_pets = config.enabled_features.contains(Feature::Pets);
    let update_lowestbin = config.enabled_features.contains(Feature::Lowestbin);
    let update_underbin = config.enabled_features.contains(Feature::Underbin);
    let update_average_auction = config.enabled_features.contains(Feature::AverageAuction);

    // Only fetch auctions if any of APIs that need the auctions are enabled
    if update_query || update_pets || update_lowestbin || update_underbin {
        // First page to get the total number of pages
        let json = get_auction_page(0).await;
        if json.is_null() || json.get("auctions").is_none() {
            error(
                "Failed to fetch the first (page=0) auction page. Canceling this run.".to_string(),
            );
            return;
        }

        // Parse the first page's auctions and append them to the prices
        parse_auctions(
            json.get("auctions").unwrap().as_array().unwrap(),
            &mut inserted_uuids,
            &mut query_prices,
            &mut pet_prices,
            &mut bin_prices,
            &mut under_bin_prices,
            &past_bin_prices,
            update_query,
            update_pets,
            update_lowestbin,
            update_underbin,
        );

        // Stores the futures for all auction page in order to utilize multithreading
        let mut futures = FuturesUnordered::new();

        let total_pages: i64 = json.get("totalPages").unwrap().as_i64().unwrap();
        let mut num_failed = 0;

        debug!("Sending {} async requests", total_pages);
        // Skip page zero since it's already been parsed
        for page_number in 1..total_pages {
            futures.push(get_auction_page(page_number));
        }
        debug!("All async requests have been sent");

        loop {
            let before_page_request = Instant::now();
            // Get the page from the Hypixel API
            match futures.next().await {
                Some(page_request) => {
                    if page_request.is_null() {
                        num_failed += 1;
                        error(format!(
                            "Failed to fetch a page with a total of {} failed page(s)",
                            num_failed
                        ));
                        continue;
                    }

                    match page_request.get("page") {
                        Some(page) => {
                            debug!("---------------- Fetching page {}", page.as_i64().unwrap());
                        }
                        None => {
                            num_failed += 1;
                            error(format!(
                                "Failed to fetch a page with a total of {} failed page(s)",
                                num_failed
                            ));
                            continue;
                        }
                    }

                    debug!(
                        "Request time: {}ms",
                        before_page_request.elapsed().as_millis()
                    );

                    // Parse the auctions and append them to the prices
                    let before_page_parse = Instant::now();
                    parse_auctions(
                        page_request.get("auctions").unwrap().as_array().unwrap(),
                        &mut inserted_uuids,
                        &mut query_prices,
                        &mut pet_prices,
                        &mut bin_prices,
                        &mut under_bin_prices,
                        &past_bin_prices,
                        update_query,
                        update_pets,
                        update_lowestbin,
                        update_underbin,
                    );
                    debug!(
                        "Parsing time: {}ms",
                        before_page_parse.elapsed().as_millis()
                    );

                    debug!(
                        "Total time: {}ms",
                        before_page_request.elapsed().as_millis()
                    );
                }
                // We have reached the last element in the vector
                None => break,
            }
        }
    }

    // Update average auctions if the feature is enabled
    if update_average_auction {
        parse_avg_auctions(&mut avg_ah_prices).await;
    }

    let fetch_sec = started.elapsed().as_secs();
    info!("Total fetch time: {}s", started.elapsed().as_secs());

    debug!("Inserting into database");
    let insert_started = Instant::now();

    // Lowest bin & under bin APIs
    if update_lowestbin {
        // Lowest bin API
        let bins_started = Instant::now();
        match update_bins_local(&bin_prices).await {
            Ok(_) => {
                info(format!(
                    "Successfully updated bins file in {}ms",
                    bins_started.elapsed().as_millis()
                ));
            }
            Err(e) => error(format!("Error updating bins file: {}", e)),
        }

        // Under bin API
        if update_underbin {
            let under_bins_started = Instant::now();
            match update_under_bins_local(&under_bin_prices).await {
                Ok(_) => {
                    info(format!(
                        "Successfully updated under bins file in {}ms",
                        under_bins_started.elapsed().as_millis()
                    ));
                }
                Err(e) => error(format!("Error updating under bins file: {}", e)),
            }
        }
    }

    // Query API
    if update_query {
        let query_started = Instant::now();
        update_query_items_local(query_prices.iter().map(|o| o.item_name.clone()).collect()).await;
        match update_query_database(query_prices).await {
            Ok(rows) => {
                info(format!(
                    "Successfully inserted {} query auctions into database in {}ms",
                    rows,
                    query_started.elapsed().as_millis()
                ));
            }
            Err(e) => error(format!("Error inserting query into database: {}", e)),
        }
    }

    // Pets API
    if update_pets {
        let pets_started = Instant::now();
        match update_pets_database(&mut pet_prices).await {
            Ok(rows) => {
                info(format!(
                    "Successfully inserted {} pets into database in {}ms",
                    rows,
                    pets_started.elapsed().as_millis()
                ));
            }
            Err(e) => error(format!("Error inserting pets into database: {}", e)),
        }
    }

    // Average auctions API
    if update_average_auction {
        let avg_ah_started = Instant::now();
        match update_avg_ah_database(avg_ah_prices, started_epoch).await {
            Ok(_) => {
                info(format!(
                    "Successfully inserted average auctions into database in {}ms",
                    avg_ah_started.elapsed().as_millis()
                ));
            }
            Err(e) => error(format!(
                "Error inserting average auctions into database: {}",
                e
            )),
        }
    }

    info(format!(
        "Fetch time: {}s | Insert time: {}s | Total time: {}s",
        fetch_sec,
        insert_started.elapsed().as_secs(),
        started.elapsed().as_secs()
    ));

    *IS_UPDATING.lock().await = false;
    *TOTAL_UPDATES.lock().await += 1;
    *LAST_UPDATED.lock().await = started_epoch;
}

/* Parses a page of auctions to a vector of documents  */
fn parse_auctions(
    auctions: &[Value],
    inserted_uuids: &mut DashSet<String>,
    query_prices: &mut Vec<DatabaseItem>,
    pet_prices: &mut DashMap<String, i64>,
    bin_prices: &mut DashMap<String, i64>,
    under_bin_prices: &mut Vec<Value>,
    past_bin_prices: &DashMap<String, i64>,
    update_query: bool,
    update_pets: bool,
    update_lowestbin: bool,
    update_underbin: bool,
) {
    for auction in auctions.iter() {
        let uuid = auction.get("uuid").unwrap().as_str().unwrap();
        // Prevent duplicate auctions
        if inserted_uuids.insert(uuid.to_string()) {
            let item_name = auction
                .get("item_name")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
            let auctioneer = auction
                .get("auctioneer")
                .unwrap()
                .as_str()
                .unwrap()
                .to_string();
            let item_lore = auction.get("item_lore").unwrap().as_str().unwrap();
            let mut tier = auction.get("tier").unwrap().as_str().unwrap();
            let starting_bid = auction.get("starting_bid").unwrap().as_i64().unwrap();
            let bin =
                auction.get("bin").is_some() && auction.get("bin").unwrap().as_bool().unwrap();
            let pet_info;

            let nbt = &to_nbt(
                serde_json::from_value(auction.get("item_bytes").unwrap().to_owned()).unwrap(),
            )
            .unwrap()
            .i[0];
            let id = &nbt.tag.extra_attributes.id;
            let mut internal_id = id.to_owned();

            // Get enchants if the item is an enchanted book
            let mut enchants = Vec::new();
            if id == "ENCHANTED_BOOK" && nbt.tag.extra_attributes.enchantments.is_some() {
                for entry in nbt.tag.extra_attributes.enchantments.as_ref().unwrap() {
                    if bin && update_lowestbin {
                        update_lower_else_insert(
                            &format!("{};{}", entry.key().to_uppercase(), entry.value()),
                            starting_bid,
                            bin_prices,
                        );
                    }
                    if update_query {
                        enchants.push(format!("{};{}", entry.key().to_uppercase(), entry.value()));
                    }
                }
            } else if id == "PET" {
                pet_info = serde_json::from_str::<Value>(
                    nbt.tag
                        .extra_attributes
                        .pet
                        .as_ref()
                        .unwrap()
                        .to_owned()
                        .as_mut_str(),
                )
                .unwrap();
                let mut tb_str = "";

                if match pet_info.get("heldItem") {
                    Some(held_item) => match held_item.as_str().unwrap() {
                        "PET_ITEM_TIER_BOOST" | "PET_ITEM_VAMPIRE_FANG" | "PET_ITEM_TOY_JERRY" => {
                            true
                        }
                        _ => false,
                    },
                    None => false,
                } {
                    // Hypixel API is weird and if the pet is tier boosted, the 'tier' field in the auction shows the rarity after boosting
                    tier = pet_info.get("tier").unwrap().as_str().unwrap();
                    tb_str = "_TB";
                }

                if bin && update_pets {
                    update_lower_else_insert(
                        &format!("{}_{}{}", item_name.replace("_✦", ""), tier, tb_str)
                            .replace(' ', "_")
                            .to_uppercase(),
                        starting_bid,
                        pet_prices,
                    );
                }

                if bin && update_lowestbin {
                    let mut split = item_name.split("] ");
                    split.next();

                    if let Some(pet_name) = split.next() {
                        internal_id = format!(
                            "{};{}",
                            pet_name.replace(' ', "_").replace("_✦", "").to_uppercase(),
                            match tier {
                                "COMMON" => 0,
                                "UNCOMMON" => 1,
                                "RARE" => 2,
                                "EPIC" => 3,
                                "LEGENDARY" => 4,
                                "MYTHIC" => 5,
                                _ => -1,
                            }
                        );
                    }
                }
            }

            if bin && update_lowestbin {
                update_lower_else_insert(&internal_id, starting_bid, bin_prices);

                if update_underbin
                    && id != "PET" // TODO: Fix pet and enchanted book under bins
                    && id != "ENCHANTED_BOOK"
                    && !item_lore.contains("Furniture")
                    && item_name != "null"
                {
                    if let Some(past_bin_price) = past_bin_prices.get(&internal_id) {
                        let profit = calculate_with_taxes(*past_bin_price.value()) - starting_bid;
                        if profit > 1000000 {
                            under_bin_prices.push(json!({
                                "uuid": uuid.to_string(),
                                "name": item_name,
                                "id" : internal_id,
                                "auctioneer": auctioneer,
                                "starting_bid" : starting_bid,
                                "past_bin_price": *past_bin_price.value(),
                                "profit": profit
                            }));
                        }
                    }
                }
            }

            // Push this auction to the array
            if update_query {
                let mut bids = Vec::new();
                for ele in auction.get("bids").unwrap().as_array().unwrap() {
                    bids.push(Bid {
                        bidder: ele.get("bidder").unwrap().as_str().unwrap().to_string(),
                        amount: ele.get("amount").unwrap().as_i64().unwrap(),
                    });
                }

                query_prices.push(DatabaseItem {
                    uuid: uuid.to_string(),
                    auctioneer,
                    end_t: auction.get("end").unwrap().as_i64().unwrap(),
                    item_name: if id == "ENCHANTED_BOOK" {
                        MC_CODE_REGEX
                            .replace_all(item_lore.split('\n').next().unwrap_or(""), "")
                            .to_string()
                    } else {
                        item_name
                    },
                    tier: tier.to_string(),
                    starting_bid,
                    item_id: id.to_string(),
                    enchants,
                    bin,
                    bids,
                });
            }
        }
    }
}

/* Parse ended auctions into Vec<AvgAh> */
async fn parse_avg_auctions(avg_ah_prices: &mut Vec<AvgAh>) {
    let page_request = get_ended_auctions().await;
    if page_request.is_null() {
        error("Failed to fetch ended auctions".to_string());
    } else {
        // Store the sum and count for each unique item id across all ended auctions
        let avg_ah_map: DashMap<String, AvgAhSum> = DashMap::new();
        for auction in page_request.get("auctions").unwrap().as_array().unwrap() {
            if auction.get("bin").unwrap().as_bool().unwrap() {
                continue;
            }

            let nbt = &to_nbt(
                serde_json::from_value(auction.get("item_bytes").unwrap().to_owned()).unwrap(),
            )
            .unwrap()
            .i[0];
            let mut id = nbt.tag.extra_attributes.id.to_owned();

            if id == "ENCHANTED_BOOK" && nbt.tag.extra_attributes.enchantments.is_some() {
                let enchants = nbt.tag.extra_attributes.enchantments.as_ref().unwrap();
                match enchants.len() {
                    1 => {
                        for entry in enchants {
                            id = format!("{};{}", entry.key().to_uppercase(), entry.value());
                        }
                    }
                    // If there is more than one enchant, the price might be higher, causing the average auction data to be incorrect
                    _ => continue,
                }
            } else if id == "PET" {
                let pet_info = serde_json::from_str::<Value>(
                    nbt.tag
                        .extra_attributes
                        .pet
                        .as_ref()
                        .unwrap()
                        .to_owned()
                        .as_mut_str(),
                )
                .unwrap();

                let item_name = MC_CODE_REGEX
                    .replace_all(&nbt.tag.display.name, "")
                    .to_string();
                let mut split = item_name.split("] ");
                split.next();

                id = format!(
                    "{};{}",
                    split
                        .next()
                        .unwrap()
                        .replace(' ', "_")
                        .replace("_✦", "")
                        .to_uppercase(),
                    match pet_info.get("tier").unwrap().as_str().unwrap() {
                        "COMMON" => 0,
                        "UNCOMMON" => 1,
                        "RARE" => 2,
                        "EPIC" => 3,
                        "LEGENDARY" => 4,
                        "MYTHIC" => 5,
                        _ => -1,
                    }
                );
            }

            let price = auction.get("price").unwrap().as_i64().unwrap();
            // If the map already has this id, then add this auction to the existing auctions, otherwise create a new entry
            if avg_ah_map.contains_key(&id) {
                avg_ah_map.alter(&id, |_, value| value.add(price));
            } else {
                avg_ah_map.insert(
                    id,
                    AvgAhSum {
                        sum: price,
                        count: 1,
                    },
                );
            }
        }

        // Average all the id's and store them in the avg_ah_prices vector
        for ele in avg_ah_map {
            avg_ah_prices.push(AvgAh {
                item_id: ele.0,
                price: (ele.1.sum as f64) / (ele.1.count as f64),
                sales: ele.1.count as f32,
            })
        }
    }
}

/* Gets an auction page from the Hypixel API */
async fn get_auction_page(page_number: i64) -> Value {
    let res = HTTP_CLIENT
        .get(format!(
            "https://api.hypixel.net/skyblock/auctions?page={}",
            page_number
        ))
        .send()
        .await;
    if res.is_ok() {
        let json = res.unwrap().body_json().await;
        if json.is_ok() {
            return json.unwrap();
        }
    }

    serde_json::Value::Null
}

/* Gets ended auctions from the Hypixel API */
async fn get_ended_auctions() -> Value {
    let res = HTTP_CLIENT
        .get("https://api.hypixel.net/skyblock/auctions_ended")
        .send()
        .await;
    if res.is_ok() {
        let json = res.unwrap().body_json().await;
        if json.is_ok() {
            return json.unwrap();
        }
    }

    serde_json::Value::Null
}
