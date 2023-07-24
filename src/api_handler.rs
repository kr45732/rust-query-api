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
use std::sync::{Arc, Mutex};
use std::{fs, time::Instant};

/// Update the enabled APIs
pub async fn update_auctions(config: Arc<Config>) -> bool {
    info(String::from("Fetching auctions..."));

    *IS_UPDATING.lock().await = true;
    let started = Instant::now();
    let mut started_epoch = get_timestamp_millis() as i64;
    let previous_started_epoch = *LAST_UPDATED.lock().await;
    // Periodically fetch entire ah to correct excess/missing auctions (please fix your API Hypixel)
    let last_updated = if *TOTAL_UPDATES.lock().await % 5 == 0 {
        0
    } else {
        previous_started_epoch
    };
    let is_first_update = last_updated == 0;

    // Stores all auction uuids in auctions vector to prevent duplicates
    let inserted_uuids: DashSet<String> = DashSet::new();
    let query_prices: Mutex<Vec<QueryDatabaseItem>> = Mutex::new(Vec::new());
    let pet_prices: DashMap<String, AvgSum> = DashMap::new();
    let bin_prices: DashMap<String, f32> = DashMap::new();
    let under_bin_prices: DashMap<String, Value> = DashMap::new();
    let avg_ah_prices: Mutex<Vec<AvgAh>> = Mutex::new(Vec::new());
    let avg_bin_prices: Mutex<Vec<AvgAh>> = Mutex::new(Vec::new());
    let past_bin_prices: DashMap<String, f32> = serde_json::from_str(
        &fs::read_to_string("lowestbin.json").unwrap_or_else(|_| String::from("{}")),
    )
    .unwrap();
    let ended_auction_uuids: DashSet<String> = DashSet::new();

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
            return true;
        }

        let json = json_opt.unwrap();
        started_epoch = json.last_updated;

        // May run too early sometimes
        if started_epoch == previous_started_epoch {
            return false;
        }

        // Parse the first page's auctions and append them to the prices
        let finished = parse_auctions(
            json.auctions,
            &inserted_uuids,
            &query_prices,
            &bin_prices,
            &under_bin_prices,
            &past_bin_prices,
            update_query,
            update_lowestbin,
            update_underbin,
            last_updated,
        );

        if is_first_update {
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
                        last_updated,
                    )
                    .boxed(),
                );
            }
        } else if !finished {
            for page_number in 1..json.total_pages {
                if process_auction_page(
                    page_number,
                    &inserted_uuids,
                    &query_prices,
                    &bin_prices,
                    &under_bin_prices,
                    &past_bin_prices,
                    update_query,
                    update_lowestbin,
                    update_underbin,
                    last_updated,
                )
                .await
                {
                    break;
                }
            }
        }
    }

    // Update average auctions if the feature is enabled
    if update_average_auction || update_average_bin || update_pets || !is_first_update {
        futures.push(
            parse_ended_auctions(
                &avg_ah_prices,
                &avg_bin_prices,
                &pet_prices,
                update_average_auction,
                update_average_bin,
                update_pets,
                &ended_auction_uuids,
                !is_first_update,
                &mut started_epoch,
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
    // Write async to database and files
    let insert_futures = FuturesUnordered::new();

    // Also updates bin and underbin (if enabled)
    if update_query {
        insert_futures.push(
            update_query_bin_underbin_fn(
                query_prices,
                ended_auction_uuids,
                is_first_update,
                &bin_prices,
                update_lowestbin,
                last_updated,
                update_underbin,
                &under_bin_prices,
            )
            .boxed(),
        );
    }

    if update_pets {
        insert_futures.push(update_pets_fn(pet_prices).boxed());
    }

    if update_average_auction {
        insert_futures.push(update_average_auction_fn(avg_ah_prices, started_epoch).boxed());
    }

    if update_average_bin {
        insert_futures.push(update_average_bin_fn(avg_bin_prices, started_epoch).boxed());
    }

    let logs: Vec<(String, String)> = insert_futures.collect().await;
    for ele in logs {
        if !ele.0.is_empty() {
            ok_logs.push_str(&ele.0);
        }
        if !ele.1.is_empty() {
            err_logs.push_str(&ele.1);
        }
    }

    if !ok_logs.is_empty() {
        info_mention(
            ok_logs.trim().to_string(),
            config.super_secret_config_option,
        );
    }

    if !err_logs.is_empty() {
        error(err_logs.trim().to_string());
    }

    info(format!(
        "Fetch time: {:.2}s | Insert time: {:.2}s | Total time: {:.2}s",
        fetch_sec,
        insert_started.elapsed().as_secs_f32(),
        started.elapsed().as_secs_f32()
    ));

    *TOTAL_UPDATES.lock().await += 1;
    *LAST_UPDATED.lock().await = started_epoch;
    *IS_UPDATING.lock().await = false;

    true
}

async fn process_auction_page(
    page_number: i32,
    inserted_uuids: &DashSet<String>,
    query_prices: &Mutex<Vec<QueryDatabaseItem>>,
    bin_prices: &DashMap<String, f32>,
    under_bin_prices: &DashMap<String, Value>,
    past_bin_prices: &DashMap<String, f32>,
    update_query: bool,
    update_lowestbin: bool,
    update_underbin: bool,
    last_updated: i64,
) -> bool {
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
        let is_finished = parse_auctions(
            page_request.auctions,
            inserted_uuids,
            query_prices,
            bin_prices,
            under_bin_prices,
            past_bin_prices,
            update_query,
            update_lowestbin,
            update_underbin,
            last_updated,
        );
        debug!(
            "Parsing time: {}ms",
            before_page_parse.elapsed().as_millis()
        );

        debug!(
            "Total time: {}ms",
            before_page_request.elapsed().as_millis()
        );

        return is_finished;
    }

    false
}

/* Parses a page of auctions and updates query, lowestbin, and underbin */
fn parse_auctions(
    auctions: Vec<Auction>,
    inserted_uuids: &DashSet<String>,
    query_prices: &Mutex<Vec<QueryDatabaseItem>>,
    bin_prices: &DashMap<String, f32>,
    under_bin_prices: &DashMap<String, Value>,
    past_bin_prices: &DashMap<String, f32>,
    update_query: bool,
    update_lowestbin: bool,
    update_underbin: bool,
    last_updated: i64,
) -> bool {
    let is_first_update = last_updated == 0;

    for auction in auctions {
        if !is_first_update && last_updated >= auction.last_updated {
            return true;
        }

        // Prevent duplicate auctions (returns false if already exists)
        if inserted_uuids.insert(auction.uuid.to_string()) {
            let mut tier = auction.tier;

            let nbt = &parse_nbt(&auction.item_bytes).unwrap().i[0];
            let extra_attrs = &nbt.tag.extra_attributes;
            let id = extra_attrs.id.to_owned();
            let mut lowestbin_id = id.to_owned();
            let mut lowestbin_price = auction.starting_bid as f32 / nbt.count as f32;

            let mut enchants = Vec::new();
            let mut attributes = Vec::new();
            if update_query {
                if let Some(enchantments) = &extra_attrs.enchantments {
                    for entry in enchantments {
                        enchants.push(format!("{};{}", entry.key().to_uppercase(), entry.value()));
                    }
                }
                if let Some(attributes_unwrap) = &extra_attrs.attributes {
                    for entry in attributes_unwrap {
                        attributes.push(format!(
                            "ATTRIBUTE_SHARD_{};{}",
                            entry.0.to_uppercase(),
                            entry.1
                        ));
                    }
                }
            }

            if id == "PET" {
                // If the pet is tier boosted, the tier field in the auction shows the rarity after boosting
                tier = serde_json::from_str::<PetInfo>(extra_attrs.pet.as_ref().unwrap())
                    .unwrap()
                    .tier;

                if auction.bin && update_lowestbin {
                    let mut split = auction.item_name.split("] ");
                    split.next();

                    if let Some(pet_name) = split.next() {
                        lowestbin_id = format!(
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
                if let Some(attributes) = &extra_attrs.attributes {
                    if id == "ATTRIBUTE_SHARD" {
                        if attributes.len() == 1 {
                            for entry in attributes {
                                lowestbin_id = format!("{}_{}", id, entry.0.to_uppercase());
                                lowestbin_price /= 2_i64.pow((entry.1 - 1) as u32) as f32;
                            }
                        }
                    } else {
                        for entry in attributes {
                            lowestbin_id.push_str("+ATTRIBUTE_SHARD_");
                            lowestbin_id.push_str(&entry.0.to_uppercase());
                        }
                    }
                }
                if id == "PARTY_HAT_CRAB" || id == "PARTY_HAT_CRAB_ANIMATED" {
                    if let Some(party_hat_color) = &extra_attrs.party_hat_color {
                        lowestbin_id = format!(
                            "PARTY_HAT_CRAB_{}{}",
                            party_hat_color.to_uppercase(),
                            if id.ends_with("_ANIMATED") {
                                "_ANIMATED"
                            } else {
                                ""
                            }
                        );
                    }
                } else if id == "PARTY_HAT_SLOTH" {
                    if let Some(party_hat_emoji) = &extra_attrs.party_hat_emoji {
                        lowestbin_id = format!("{}_{}", id, party_hat_emoji.to_uppercase());
                    }
                } else if id == "NEW_YEAR_CAKE" {
                    if let Some(new_years_cake) = &extra_attrs.new_years_cake {
                        lowestbin_id = format!("{}_{}", id, new_years_cake);
                    }
                } else if id == "MIDAS_SWORD" || id == "MIDAS_STAFF" {
                    if let Some(winning_bid) = &extra_attrs.winning_bid {
                        let best_bid = if id == "MIDAS_SWORD" {
                            50000000
                        } else {
                            100000000
                        };
                        if winning_bid > &best_bid {
                            lowestbin_id = format!("{}_{}", id, best_bid);
                        }
                    }
                } else if id == "RUNE" {
                    if let Some(runes) = &extra_attrs.runes {
                        if runes.len() == 1 {
                            for entry in runes {
                                lowestbin_id = format!(
                                    "{}_RUNE;{}",
                                    entry.key().to_uppercase(),
                                    entry.value()
                                );
                            }
                        }
                    }
                }

                if is_first_update {
                    update_lower_else_insert(&lowestbin_id, lowestbin_price, bin_prices);
                }

                if update_underbin
                    && id != "PET" // TODO: Improve under bins
                    && !auction.item_lore.contains("Furniture")
                    &&  auction.item_name != "null"
                    && !auction.item_name.contains("Minion Skin")
                {
                    if let Some(past_bin_price) = past_bin_prices.get(&lowestbin_id) {
                        let profit = calculate_with_taxes(*past_bin_price.value())
                            - auction.starting_bid as f32;
                        if profit > 1000000.0 {
                            under_bin_prices.insert(
                                auction.uuid.clone(),
                                json!({
                                    "uuid": auction.uuid,
                                    "name":  auction.item_name,
                                    "id" : lowestbin_id,
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
                    score: None,
                    auctioneer: auction.auctioneer,
                    end_t: auction.end,
                    item_name: auction.item_name,
                    lore: format!("{}\n{}", nbt.tag.display.name, auction.item_lore),
                    tier: tier.to_string(),
                    starting_bid: auction.starting_bid,
                    highest_bid: auction.highest_bid_amount,
                    lowestbin_price,
                    item_id: id,
                    internal_id: lowestbin_id,
                    enchants,
                    attributes,
                    bin: auction.bin,
                    bids,
                    count: nbt.count,
                    potato_books: extra_attrs.hot_potato_count,
                    stars: extra_attrs.get_stars(),
                    farming_for_dummies: extra_attrs.farming_for_dummies_count,
                    transmission_tuner: extra_attrs.tuned_transmission,
                    mana_disintegrator: extra_attrs.mana_disintegrator_count,
                    reforge: extra_attrs.modifier.to_owned(),
                    rune: extra_attrs.get_rune(),
                    skin: extra_attrs.skin.to_owned(),
                    power_scroll: extra_attrs.power_ability_scroll.to_owned(),
                    drill_upgrade_module: extra_attrs.drill_part_upgrade_module.to_owned(),
                    drill_fuel_tank: extra_attrs.drill_part_fuel_tank.to_owned(),
                    drill_engine: extra_attrs.drill_part_engine.to_owned(),
                    dye: extra_attrs.dye_item.to_owned(),
                    accessory_enrichment: extra_attrs.get_talisman_enrichment(),
                    recombobulated: extra_attrs.is_recombobulated(),
                    wood_singularity: extra_attrs.is_wood_singularity_applied(),
                    art_of_war: extra_attrs.is_art_of_war_applied(),
                    art_of_peace: extra_attrs.is_art_of_peace_applied(),
                    etherwarp: extra_attrs.is_etherwarp_applied(),
                    necron_scrolls: extra_attrs.ability_scroll.to_owned(),
                    gemstones: extra_attrs.get_gemstones(),
                });
            }
        }
    }

    false
}

/* Parse ended auctions into Vec<AvgAh> */
async fn parse_ended_auctions(
    avg_ah_prices: &Mutex<Vec<AvgAh>>,
    avg_bin_prices: &Mutex<Vec<AvgAh>>,
    pet_prices: &DashMap<String, AvgSum>,
    update_average_auction: bool,
    update_average_bin: bool,
    update_pets: bool,
    ended_auction_uuids: &DashSet<String>,
    update_ended_auction_uuids: bool,
    started_epoch: &mut i64,
) -> bool {
    match get_ended_auctions().await {
        Some(page_request) => {
            *started_epoch = page_request.last_updated;

            let avg_ah_map: DashMap<String, AvgSum> = DashMap::new();
            let avg_bin_map: DashMap<String, AvgSum> = DashMap::new();

            for mut auction in page_request.auctions {
                if update_ended_auction_uuids {
                    ended_auction_uuids.insert(auction.auction_id);
                }

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
                let extra_attrs = &nbt.tag.extra_attributes;
                let mut id = extra_attrs.id.to_owned();

                if id == "PET" {
                    let pet_info =
                        serde_json::from_str::<PetInfo>(extra_attrs.pet.as_ref().unwrap()).unwrap();

                    let item_name = MC_CODE_REGEX
                        .replace_all(&nbt.tag.display.name, "")
                        .to_string();

                    if update_pets {
                        let pet_id = format!(
                            "{}_{}{}",
                            item_name.replace(' ', "_").replace("_✦", ""),
                            pet_info.tier,
                            if let Some(held_item) = pet_info.held_item {
                                if held_item == "PET_ITEM_TIER_BOOST" {
                                    "_TB"
                                } else {
                                    ""
                                }
                            } else {
                                ""
                            }
                        )
                        .to_uppercase();

                        if pet_prices.contains_key(&pet_id) {
                            pet_prices.alter(&pet_id, |_, value| value.update(auction.price, 1));
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

                if let Some(attributes) = &extra_attrs.attributes {
                    if id == "ATTRIBUTE_SHARD" {
                        if attributes.len() == 1 {
                            for entry in attributes {
                                id = format!("ATTRIBUTE_SHARD_{}", entry.0.to_uppercase());
                                auction.price /= 2_i64.pow((entry.1 - 1) as u32);
                            }
                        }
                    } else if !attributes.is_empty() {
                        // Track average of item (regardless of attributes)
                        if update_average_bin && auction.bin {
                            update_average_map(&avg_bin_map, &id, auction.price, nbt.count);
                        } else if update_average_auction && !auction.bin {
                            update_average_map(&avg_ah_map, &id, auction.price, nbt.count);
                        }

                        for entry in attributes {
                            id.push_str("+ATTRIBUTE_SHARD_");
                            id.push_str(&entry.0.to_uppercase());
                        }
                    }
                }
                if id == "PARTY_HAT_CRAB" || id == "PARTY_HAT_CRAB_ANIMATED" {
                    if let Some(party_hat_color) = &extra_attrs.party_hat_color {
                        id = format!(
                            "PARTY_HAT_CRAB_{}{}",
                            party_hat_color.to_uppercase(),
                            if id.ends_with("_ANIMATED") {
                                "_ANIMATED"
                            } else {
                                ""
                            }
                        );
                    }
                } else if id == "PARTY_HAT_SLOTH" {
                    if let Some(party_hat_emoji) = &extra_attrs.party_hat_emoji {
                        id = format!("{}_{}", id, party_hat_emoji.to_uppercase());
                    }
                } else if id == "NEW_YEAR_CAKE" {
                    if let Some(new_years_cake) = &extra_attrs.new_years_cake {
                        id = format!("{}_{}", id, new_years_cake);
                    }
                } else if id == "MIDAS_SWORD" || id == "MIDAS_STAFF" {
                    if let Some(winning_bid) = &extra_attrs.winning_bid {
                        let best_bid = if id == "MIDAS_SWORD" {
                            50000000
                        } else {
                            100000000
                        };
                        if winning_bid > &best_bid {
                            id = format!("{}_{}", id, best_bid);
                        }
                    }
                } else if id == "RUNE" {
                    if let Some(runes) = &extra_attrs.runes {
                        if runes.len() == 1 {
                            for entry in runes {
                                id = format!(
                                    "{}_RUNE;{}",
                                    entry.key().to_uppercase(),
                                    entry.value()
                                );
                            }
                        }
                    }
                }

                if update_average_bin && auction.bin {
                    update_average_map(&avg_bin_map, &id, auction.price, nbt.count);
                } else if update_average_auction && !auction.bin {
                    update_average_map(&avg_ah_map, &id, auction.price, nbt.count);
                }
            }

            // Average all the averaged auctions and store them in the avg_ah_prices vector
            for ele in avg_ah_map {
                avg_ah_prices.lock().unwrap().push(AvgAh {
                    item_id: ele.0,
                    price: (ele.1.sum as f32) / (ele.1.count as f32),
                    sales: ele.1.count as f32,
                })
            }

            // Average all the averaged bins and store them in the avg_bin_prices vector
            for ele in avg_bin_map {
                avg_bin_prices.lock().unwrap().push(AvgAh {
                    item_id: ele.0,
                    price: (ele.1.sum as f32) / (ele.1.count as f32),
                    sales: ele.1.count as f32,
                })
            }
        }
        None => {
            error(String::from("Failed to fetch ended auctions"));
        }
    }

    true
}

/* Gets an auction page from the Hypixel API */
async fn get_auction_page(page_number: i32) -> Option<Auctions> {
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
