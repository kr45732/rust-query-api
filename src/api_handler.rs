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

use crate::config::{Config, Feature};
use crate::{statics::*, structs::*, utils::*};
use dashmap::{DashMap, DashSet};
use futures::FutureExt;
use futures::{stream::FuturesUnordered, StreamExt};
use log::{debug, info};
use serde_json::{json, Value};
use std::fmt::Write;
use std::sync::{Arc, Mutex};
use std::{fs, time::Instant};

/// Update the enabled APIs
pub async fn update_auctions(config: Arc<Config>) {
    info(String::from("Fetching auctions..."));

    let started = Instant::now();
    let started_epoch = get_timestamp_millis() as i64;
    *IS_UPDATING.lock().await = true;

    // Stores all auction uuids in auctions vector to prevent duplicates
    let inserted_uuids: DashSet<String> = DashSet::new();
    let query_prices: Mutex<Vec<QueryDatabaseItem>> = Mutex::new(Vec::new());
    let pet_prices: DashMap<String, AvgSum> = DashMap::new();
    let bin_prices: DashMap<String, f64> = DashMap::new();
    let under_bin_prices: DashMap<String, Value> = DashMap::new();
    let avg_ah_prices: Mutex<Vec<AvgAh>> = Mutex::new(Vec::new());
    let avg_bin_prices: Mutex<Vec<AvgAh>> = Mutex::new(Vec::new());
    let past_bin_prices: DashMap<String, f64> = serde_json::from_str(
        &fs::read_to_string("lowestbin.json").unwrap_or_else(|_| String::from("{}")),
    )
    .unwrap();

    // Get which APIs to update
    let update_query = config.is_enabled(Feature::Query);
    let update_pets = config.is_enabled(Feature::Pets);
    let update_lowestbin = config.is_enabled(Feature::Lowestbin);
    let update_underbin = config.is_enabled(Feature::Underbin);
    let update_average_auction = config.is_enabled(Feature::AverageAuction);
    let update_average_bin = config.is_enabled(Feature::AverageBin);

    // Stores the futures for all auction pages in order to utilize multithreading
    let futures = FuturesUnordered::new();

    // Only fetch auctions if any of APIs that need the auctions are enabled
    if update_query || update_lowestbin || update_underbin {
        // First page to get the total number of pages
        let json_opt = get_auction_page(0).await;
        if json_opt.is_none() {
            error(String::from(
                "Failed to fetch the first auction page. Canceling this run.",
            ));
            return;
        }

        let json = json_opt.unwrap();
        // Parse the first page's auctions and append them to the prices
        parse_auctions(
            json.auctions,
            &inserted_uuids,
            &query_prices,
            &bin_prices,
            &under_bin_prices,
            &past_bin_prices,
            update_query,
            update_lowestbin,
            update_underbin,
        );

        debug!("Sending {} async requests", json.total_pages);
        // Skip page zero since it's already been parsed
        for page_number in 1..json.total_pages {
            futures.push(
                process_auction_page(
                    page_number,
                    &inserted_uuids,
                    &query_prices,
                    &bin_prices,
                    &under_bin_prices,
                    &past_bin_prices,
                    update_query,
                    update_lowestbin,
                    update_underbin,
                )
                .boxed(),
            );
        }
    }

    // Update average auctions if the feature is enabled
    if update_average_auction || update_average_bin || update_pets {
        futures.push(
            parse_ended_auctions(
                &avg_ah_prices,
                &avg_bin_prices,
                &pet_prices,
                update_average_auction,
                update_average_bin,
                update_pets,
            )
            .boxed(),
        );
    }

    let _: Vec<_> = futures.collect().await;

    let fetch_sec = started.elapsed().as_secs_f32();
    info!("Total fetch time: {:.2}s", fetch_sec);

    debug!("Inserting into database");
    let insert_started = Instant::now();
    let mut ok_logs = String::new();
    let mut err_logs = String::new();

    if update_lowestbin {
        let bins_started = Instant::now();
        let _ = match update_bins_local(&bin_prices).await {
            Ok(_) => write!(
                ok_logs,
                "Successfully updated bins file in {}ms",
                bins_started.elapsed().as_millis()
            ),
            Err(e) => write!(err_logs, "Error updating bins file: {}", e),
        };

        if update_underbin {
            let under_bins_started = Instant::now();
            let _ = match update_under_bins_local(&under_bin_prices).await {
                Ok(_) => write!(
                    ok_logs,
                    "\nSuccessfully updated under bins file in {}ms",
                    under_bins_started.elapsed().as_millis()
                ),
                Err(e) => write!(err_logs, "\nError updating under bins file: {}", e),
            };
        }
    }

    if update_query {
        let query_started = Instant::now();
        update_query_items_local(&query_prices).await;
        let _ = match update_query_database(query_prices).await {
            Ok(rows) => write!(
                ok_logs,
                "\nSuccessfully inserted {} query auctions into database in {}ms",
                rows,
                query_started.elapsed().as_millis()
            ),
            Err(e) => write!(err_logs, "\nError inserting query into database: {}", e),
        };
    }

    if update_pets {
        let pets_started = Instant::now();
        let _ = match update_pets_database(pet_prices).await {
            Ok(rows) => write!(
                ok_logs,
                "\nSuccessfully inserted {} pets into database in {}ms",
                rows,
                pets_started.elapsed().as_millis()
            ),
            Err(e) => write!(err_logs, "\nError inserting pets into database: {}", e),
        };
    }

    if update_average_auction {
        let avg_ah_started = Instant::now();
        let _ = match update_avg_ah_database(avg_ah_prices, started_epoch).await {
            Ok(_) => write!(
                ok_logs,
                "\nSuccessfully inserted average auctions into database in {}ms",
                avg_ah_started.elapsed().as_millis()
            ),
            Err(e) => write!(
                err_logs,
                "\nError inserting average auctions into database: {}",
                e,
            ),
        };
    }

    if update_average_bin {
        let avg_bin_started = Instant::now();
        let _ = match update_avg_bin_database(avg_bin_prices, started_epoch).await {
            Ok(_) => write!(
                ok_logs,
                "\nSuccessfully inserted average bins into database in {}ms",
                avg_bin_started.elapsed().as_millis()
            ),
            Err(e) => write!(
                err_logs,
                "\nError inserting average bins into database: {}",
                e
            ),
        };
    }

    if !ok_logs.is_empty() {
        info_mention(ok_logs, config.super_secret_config_option);
    }

    if !err_logs.is_empty() {
        error(err_logs);
    }

    info(format!(
        "Fetch time: {:.2}s | Insert time: {:.2}s | Total time: {:.2}s",
        fetch_sec,
        insert_started.elapsed().as_secs_f32(),
        started.elapsed().as_secs_f32()
    ));

    *IS_UPDATING.lock().await = false;
    *TOTAL_UPDATES.lock().await += 1;
    *LAST_UPDATED.lock().await = started_epoch;
}

async fn process_auction_page(
    page_number: i64,
    inserted_uuids: &DashSet<String>,
    query_prices: &Mutex<Vec<QueryDatabaseItem>>,
    bin_prices: &DashMap<String, f64>,
    under_bin_prices: &DashMap<String, Value>,
    past_bin_prices: &DashMap<String, f64>,
    update_query: bool,
    update_lowestbin: bool,
    update_underbin: bool,
) {
    let before_page_request = Instant::now();
    // Get the page from the Hypixel API
    if let Some(page_request) = get_auction_page(page_number).await {
        debug!("---------------- Fetching page {}", page_request.page);
        debug!(
            "Request time: {}ms",
            before_page_request.elapsed().as_millis()
        );

        // Parse the auctions and append them to the prices
        let before_page_parse = Instant::now();
        parse_auctions(
            page_request.auctions,
            inserted_uuids,
            query_prices,
            bin_prices,
            under_bin_prices,
            past_bin_prices,
            update_query,
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
}

/* Parses a page of auctions and updates query, lowestbin, and underbin */
fn parse_auctions(
    auctions: Vec<Auction>,
    inserted_uuids: &DashSet<String>,
    query_prices: &Mutex<Vec<QueryDatabaseItem>>,
    bin_prices: &DashMap<String, f64>,
    under_bin_prices: &DashMap<String, Value>,
    past_bin_prices: &DashMap<String, f64>,
    update_query: bool,
    update_lowestbin: bool,
    update_underbin: bool,
) {
    for auction in auctions {
        // Prevent duplicate auctions (returns false if already exists)
        if inserted_uuids.insert(auction.uuid.to_string()) {
            let mut tier = auction.tier;

            let nbt = &parse_nbt(&auction.item_bytes).unwrap().i[0];
            let item_id = nbt.tag.extra_attributes.id.to_owned();
            let mut internal_id = item_id.to_owned();

            let mut enchants = Vec::new();
            if update_query && nbt.tag.extra_attributes.enchantments.is_some() {
                for entry in nbt.tag.extra_attributes.enchantments.as_ref().unwrap() {
                    enchants.push(format!("{};{}", entry.key().to_uppercase(), entry.value()));
                }
            }

            if item_id == "PET" {
                // If the pet is tier boosted, the tier field in the auction shows the rarity after boosting
                tier =
                    serde_json::from_str::<PetInfo>(nbt.tag.extra_attributes.pet.as_ref().unwrap())
                        .unwrap()
                        .tier;

                if auction.bin && update_lowestbin {
                    let mut split = auction.item_name.split("] ");
                    split.next();

                    if let Some(pet_name) = split.next() {
                        internal_id = format!(
                            "{};{}",
                            pet_name.replace(' ', "_").replace("_✦", "").to_uppercase(),
                            match tier.as_str() {
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

            if auction.bin && update_lowestbin {
                let mut lowestbin_price = auction.starting_bid as f64 / nbt.count as f64;
                if item_id == "ATTRIBUTE_SHARD" {
                    if let Some(attributes) = &nbt.tag.extra_attributes.attributes {
                        if attributes.len() == 1 {
                            for entry in attributes {
                                internal_id =
                                    format!("ATTRIBUTE_SHARD_{}", entry.key().to_uppercase());
                                lowestbin_price /= 2_i64.pow((entry.value() - 1) as u32) as f64;
                            }
                        }
                    }
                } else if item_id == "PARTY_HAT_CRAB" || item_id == "PARTY_HAT_CRAB_ANIMATED" {
                    if let Some(party_hat_color) = &nbt.tag.extra_attributes.party_hat_color {
                        internal_id = format!("{}_{}", item_id, party_hat_color.to_uppercase());
                    }
                } else if item_id == "NEW_YEAR_CAKE" {
                    if let Some(new_years_cake) = &nbt.tag.extra_attributes.new_years_cake {
                        internal_id = format!("{}_{}", item_id, new_years_cake);
                    }
                }

                update_lower_else_insert(&internal_id, lowestbin_price, bin_prices);

                if update_underbin
                    && item_id != "PET" // TODO: Improve under bins
                    && !auction.item_lore.contains("Furniture")
                    &&  auction.item_name != "null"
                    && !auction.item_name.contains("Minion Skin")
                {
                    if let Some(past_bin_price) = past_bin_prices.get(&internal_id) {
                        let profit = calculate_with_taxes(*past_bin_price.value())
                            - auction.starting_bid as f64;
                        if profit > 1000000.0 {
                            under_bin_prices.insert(
                                auction.uuid.clone(),
                                json!({
                                    "uuid": auction.uuid,
                                    "name":  auction.item_name,
                                    "id" : internal_id,
                                    "auctioneer":  auction.auctioneer,
                                    "starting_bid" :  auction.starting_bid,
                                    "past_bin_price": *past_bin_price.value(),
                                    "profit": profit
                                }),
                            );
                        }
                    }
                }
            }

            // Push this auction to the array
            if update_query {
                let mut bids = Vec::new();
                for ele in auction.bids {
                    bids.push(Bid {
                        bidder: ele.bidder,
                        amount: ele.amount,
                    });
                }

                query_prices.lock().unwrap().push(QueryDatabaseItem {
                    uuid: auction.uuid,
                    auctioneer: auction.auctioneer,
                    end_t: auction.end,
                    item_name: auction.item_name,
                    tier: tier.to_string(),
                    starting_bid: if auction.bin {
                        auction.starting_bid
                    } else {
                        auction.highest_bid_amount
                    },
                    item_id,
                    enchants,
                    bin: auction.bin,
                    bids,
                    count: nbt.count,
                });
            }
        }
    }
}

/* Parse ended auctions into Vec<AvgAh> */
async fn parse_ended_auctions(
    avg_ah_prices: &Mutex<Vec<AvgAh>>,
    avg_bin_prices: &Mutex<Vec<AvgAh>>,
    pet_prices: &DashMap<String, AvgSum>,
    update_average_auction: bool,
    update_average_bin: bool,
    update_pets: bool,
) {
    match get_ended_auctions().await {
        Some(page_request) => {
            let avg_ah_map: DashMap<String, AvgSum> = DashMap::new();
            let avg_bin_map: DashMap<String, AvgSum> = DashMap::new();

            for mut auction in page_request.auctions {
                // Always update if pets is enabled, otherwise check if only auction or bin are enabled
                if !update_pets && !(update_average_auction && update_average_bin) {
                    // Only update avg ah is enabled but is bin or only update avg bin is enabled but isn't bin
                    if (update_average_auction && auction.bin)
                        || (update_average_bin && !auction.bin)
                    {
                        continue;
                    }
                }

                let nbt = &parse_nbt(&auction.item_bytes).unwrap().i[0];
                let mut id = nbt.tag.extra_attributes.id.to_owned();

                if id == "PET" {
                    let pet_info = serde_json::from_str::<PetInfo>(
                        nbt.tag.extra_attributes.pet.as_ref().unwrap(),
                    )
                    .unwrap();

                    let item_name = MC_CODE_REGEX
                        .replace_all(&nbt.tag.display.name, "")
                        .to_string();

                    if update_pets {
                        let pet_id = format!(
                            "{}_{}{}",
                            item_name.replace(' ', "_").replace("_✦", ""),
                            pet_info.tier,
                            if let Some(held_item) = pet_info.held_item {
                                match held_item.as_str() {
                                    "PET_ITEM_TIER_BOOST"
                                    | "PET_ITEM_VAMPIRE_FANG"
                                    | "PET_ITEM_TOY_JERRY" => "_TB",
                                    _ => "",
                                }
                            } else {
                                ""
                            }
                        )
                        .to_uppercase();

                        if pet_prices.contains_key(&pet_id) {
                            pet_prices.alter(&pet_id, |_, value| value.add(auction.price));
                        } else {
                            pet_prices.insert(
                                pet_id,
                                AvgSum {
                                    sum: auction.price,
                                    count: 1,
                                },
                            );
                        }
                    }

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
                        match pet_info.tier.as_str() {
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

                if !update_average_bin && !update_average_auction {
                    continue;
                }

                if id == "ATTRIBUTE_SHARD" {
                    if let Some(attributes) = &nbt.tag.extra_attributes.attributes {
                        if attributes.len() == 1 {
                            for entry in attributes {
                                id = format!("ATTRIBUTE_SHARD_{}", entry.key().to_uppercase());
                                auction.price /= 2_i64.pow((entry.value() - 1) as u32);
                            }
                        }
                    }
                } else if id == "PARTY_HAT_CRAB" || id == "PARTY_HAT_CRAB_ANIMATED" {
                    if let Some(party_hat_color) = &nbt.tag.extra_attributes.party_hat_color {
                        id = format!("{}_{}", id, party_hat_color.to_uppercase());
                    }
                } else if id == "NEW_YEAR_CAKE" {
                    if let Some(new_years_cake) = &nbt.tag.extra_attributes.new_years_cake {
                        id = format!("{}_{}", id, new_years_cake);
                    }
                }

                // If the map already has this id, then add to the existing elements, otherwise create a new entry
                if update_average_bin && auction.bin {
                    if avg_bin_map.contains_key(&id) {
                        avg_bin_map
                            .alter(&id, |_, value| value.add_multiple(auction.price, nbt.count));
                    } else {
                        avg_bin_map.insert(
                            id,
                            AvgSum {
                                sum: auction.price,
                                count: nbt.count,
                            },
                        );
                    }
                } else if update_average_auction && !auction.bin {
                    if avg_ah_map.contains_key(&id) {
                        avg_ah_map
                            .alter(&id, |_, value| value.add_multiple(auction.price, nbt.count));
                    } else {
                        avg_ah_map.insert(
                            id,
                            AvgSum {
                                sum: auction.price,
                                count: nbt.count,
                            },
                        );
                    }
                }
            }

            // Average all the averaged auctions and store them in the avg_ah_prices vector
            for ele in avg_ah_map {
                avg_ah_prices.lock().unwrap().push(AvgAh {
                    item_id: ele.0,
                    price: (ele.1.sum as f64) / (ele.1.count as f64),
                    sales: ele.1.count as f32,
                })
            }

            // Average all the averaged bins and store them in the avg_bin_prices vector
            for ele in avg_bin_map {
                avg_bin_prices.lock().unwrap().push(AvgAh {
                    item_id: ele.0,
                    price: (ele.1.sum as f64) / (ele.1.count as f64),
                    sales: ele.1.count as f32,
                })
            }
        }
        None => {
            error(String::from("Failed to fetch ended auctions"));
        }
    }
}

/* Gets an auction page from the Hypixel API */
async fn get_auction_page(page_number: i64) -> Option<Auctions> {
    let res = HTTP_CLIENT
        .get(format!(
            "https://api.hypixel.net/skyblock/auctions?page={}",
            page_number
        ))
        .send()
        .await;
    if res.is_ok() {
        res.unwrap().body_json().await.ok()
    } else {
        None
    }
}

/* Gets ended auctions from the Hypixel API */
async fn get_ended_auctions() -> Option<EndedAuctions> {
    let res = HTTP_CLIENT
        .get("https://api.hypixel.net/skyblock/auctions_ended")
        .send()
        .await;
    if res.is_ok() {
        res.unwrap().body_json().await.ok()
    } else {
        None
    }
}
