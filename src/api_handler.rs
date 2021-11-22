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
use chrono::Utc;
use dashmap::{DashMap, DashSet};
use futures::{stream::FuturesUnordered, StreamExt};
use log::debug;
use serde_json::Value;
use std::time::Instant;

/* Gets all pages of auctions from the Hypixel API and inserts them into the database */
pub async fn update_api() {
    info("Fetching auctions...".to_string()).await;

    let started = Instant::now();
    *IS_UPDATING.lock().unwrap() = true;

    // Stores all the query prices
    let mut query_prices: Vec<DatabaseItem> = Vec::new();
    // Stores all auction uuids in auctions vector to prevent duplicates
    let mut inserted_uuids: DashSet<String> = DashSet::new();
    // Stores all pet prices
    let mut pet_prices: DashMap<String, i64> = DashMap::new();
    // Stores all bin prices
    let mut bin_prices: DashMap<String, i64> = DashMap::new();

    // Which APIs to update
    let update_query = *ENABLE_QUERY.lock().unwrap();
    let update_pets = *ENABLE_PETS.lock().unwrap();
    let update_lowestbin = *ENABLE_LOWESTBIN.lock().unwrap();

    // First page to get the total number of pages
    let json = get_auction_page(0).await;
    if json.is_null() {
        error("Failed to fetch the first (page=0) auction page".to_string()).await;
        return;
    }

    parse_auctions(
        json.get("auctions").unwrap().as_array().unwrap(),
        &mut inserted_uuids,
        &mut query_prices,
        &mut pet_prices,
        &mut bin_prices,
        update_query,
        update_pets,
        update_lowestbin,
    );

    let mut futures = FuturesUnordered::new();
    let total_pages: i64 = json.get("totalPages").unwrap().as_i64().unwrap();
    let mut num_failed = 0;

    debug!("Sending {} async requests", total_pages);
    for page_number in 1..total_pages {
        let future = get_auction_page(page_number);
        futures.push(future);
    }
    debug!("All async requests have been sent");

    loop {
        let before_page_request = Instant::now();
        // Get the page from the Hypixel API
        match futures.next().await {
            Some(page_request) => {
                let page_number = page_request.get("page").unwrap().as_i64().unwrap();
                debug!("---------------- Fetching page {}", page_number);
                if page_request.is_null() {
                    num_failed += 1;
                    error(format!(
                        "Failed to fetch page {} with a total of {} failed pages",
                        page_number, num_failed
                    ))
                    .await;
                    continue;
                }
                debug!(
                    "Request time: {}ms",
                    before_page_request.elapsed().as_millis()
                );

                // Parse the auctions and add them to the auctions array
                let before_page_parse = Instant::now();
                parse_auctions(
                    page_request.get("auctions").unwrap().as_array().unwrap(),
                    &mut inserted_uuids,
                    &mut query_prices,
                    &mut pet_prices,
                    &mut bin_prices,
                    update_query,
                    update_pets,
                    update_lowestbin,
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
            None => break,
        }
    }

    info(format!(
        "Total fetch time: {}s",
        started.elapsed().as_secs()
    ))
    .await;

    // Update the auctions in the database
    debug!("Inserting into database");

    // Query API
    if update_query {
        match update_query_database(query_prices).await {
            Ok(_) => {
                info("Successfully inserted query into database".to_string()).await;
            }
            Err(e) => error(format!("Error inserting query into database: {}", e)).await,
        }
    }

    // Pets API
    if update_pets {
        match update_pets_database(&mut pet_prices).await {
            Ok(_) => {
                info("Successfully inserted pets into database".to_string()).await;
            }
            Err(e) => error(format!("Error inserting pets into database: {}", e)).await,
        }
    }

    // Bins API
    if update_lowestbin {
        match update_bins_local(&mut bin_prices).await {
            Ok(_) => {
                info("Successfully updated bins file".to_string()).await;
            }
            Err(e) => error(format!("Error updating bins file: {}", e)).await,
        }
    }

    info(format!(
        "Total fetch and insert time: {}s",
        started.elapsed().as_secs()
    ))
    .await;

    *IS_UPDATING.lock().unwrap() = false;
    *TOTAL_UPDATES.lock().unwrap() += 1;
    *LAST_UPDATED.lock().unwrap() = Utc::now().timestamp_millis();
}

/* Parses a page of auctions to a vector of documents  */
fn parse_auctions(
    auctions: &Vec<serde_json::Value>,
    inserted_uuids: &mut DashSet<String>,
    query_prices: &mut Vec<DatabaseItem>,
    pet_prices: &mut DashMap<String, i64>,
    bin_prices: &mut DashMap<String, i64>,
    update_query: bool,
    update_pets: bool,
    update_lowestbin: bool,
) {
    for auction in auctions.into_iter() {
        // Only bins for now
        if auction.get("bin").is_some() {
            let uuid = auction.get("uuid").unwrap().as_str().unwrap();
            let item_name = auction.get("item_name").unwrap().as_str().unwrap();
            let tier = auction.get("tier").unwrap().as_str().unwrap();
            let starting_bid = auction.get("starting_bid").unwrap().as_i64().unwrap();

            // Prevent duplicate auctions
            if inserted_uuids.insert(uuid.to_string()) {
                // Parse the auction's nbt
                let nbt = &to_nbt(
                    serde_json::from_value(auction.get("item_bytes").unwrap().to_owned()).unwrap(),
                )
                .unwrap()
                .i[0];
                // Item id
                let id = &nbt.tag.extra_attributes.id;

                // Get enchants if the item is an enchanted book
                let mut enchants = Vec::new();
                if id == "ENCHANTED_BOOK" && nbt.tag.extra_attributes.enchantments.is_some() {
                    for entry in nbt.tag.extra_attributes.enchantments.as_ref().unwrap() {
                        if update_lowestbin {
                            update_lower_else_insert(
                                &format!("{};{}", entry.key().to_uppercase(), entry.value()),
                                starting_bid,
                                bin_prices,
                            );
                        }
                        if update_query {
                            enchants.push(format!(
                                "{};{}",
                                entry.key().to_uppercase(),
                                entry.value()
                            ));
                        }
                    }
                } else if id == "PET" {
                    // Pets API
                    if update_pets {
                        let pet_info: Value =
                            serde_json::from_str(nbt.tag.extra_attributes.pet.as_ref().unwrap())
                                .unwrap();

                        let pet_name = &mut format!("{}_{}", item_name.replace("✦", ""), tier)
                            .replace(" ", "_")
                            .to_uppercase();
                        if match pet_info.get("heldItem") {
                            Some(held_item) => held_item.as_str().unwrap() == "PET_ITEM_TIER_BOOST",
                            None => false,
                        } {
                            pet_name.push_str("_TB");
                        }

                        update_lower_else_insert(pet_name, starting_bid, pet_prices);
                    }

                    if update_lowestbin {
                        let mut split = item_name.split("] ");
                        split.next();

                        update_lower_else_insert(
                            &format!(
                                "{};{}",
                                split.next().unwrap().replace(" ", "_").to_uppercase(),
                                match tier {
                                    "COMMON" => 0,
                                    "UNCOMMON" => 1,
                                    "RARE" => 2,
                                    "EPIC" => 3,
                                    "LEGENDARY" => 4,
                                    "MYTHIC" => 5,
                                    _ => -1,
                                }
                            ),
                            starting_bid,
                            bin_prices,
                        );
                    }
                } else {
                    if update_lowestbin {
                        update_lower_else_insert(id, starting_bid, bin_prices);
                    }
                }

                // Push this auction to the array
                if update_query {
                    query_prices.push(DatabaseItem {
                        uuid: uuid.to_string(),
                        auctioneer: auction
                            .get("auctioneer")
                            .unwrap()
                            .as_str()
                            .unwrap()
                            .to_string(),
                        end_t: auction.get("end").unwrap().as_i64().unwrap(),
                        item_name: if id != "ENCHANTED_BOOK" {
                            item_name.to_string()
                        } else {
                            MC_CODE_REGEX
                                .replace_all(
                                    auction
                                        .get("item_lore")
                                        .unwrap()
                                        .as_str()
                                        .unwrap()
                                        .split("\n")
                                        .next()
                                        .unwrap_or(""),
                                    "",
                                )
                                .to_string()
                        },
                        tier: tier.to_string(),
                        starting_bid,
                        item_id: id.to_string(),
                        enchants,
                    });
                }
            }
        }
    }
}

fn update_lower_else_insert(id: &String, starting_bid: i64, prices: &mut DashMap<String, i64>) {
    if let Some(mut ele) = prices.get_mut(id) {
        if starting_bid < *ele {
            *ele = starting_bid;
            return;
        }
    }

    prices.insert(id.clone(), starting_bid);
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
        let text = res.unwrap().text().await;
        if text.is_ok() {
            let json = serde_json::from_str(&text.unwrap());
            if json.is_ok() {
                return json.unwrap();
            }
        }
    }

    Value::Null
}
