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

use crate::statics::*;
use crate::structs::*;
use crate::utils::error;
use crate::utils::info;
use futures::pin_mut;
use log::debug;
use std::time::Instant;
use tokio_postgres::binary_copy::BinaryCopyInWriter;
use tokio_postgres::types::{ToSql, Type};

/* Gets all pages of auctions from the Hypixel API and inserts them into the database */
pub async fn fetch_auctions() {
    info("Fetching auctions...".to_string()).await;

    let started = Instant::now();
    *IS_UPDATING.lock().unwrap() = true;

    // Stores all the auctions
    let mut auctions: Vec<DatabaseItem> = Vec::new();

    // First page to get the total number of pages
    let r = get_auction_page(1).await;
    if r.is_none() {
        error("Failed to fetch the first auction page".to_string()).await;
        return;
    }
    let json = r.unwrap();
    auctions.append(&mut parse_hypixel(json.auctions));

    let mut num_failed = 0;
    for page_number in 2..json.total_pages {
        debug!("---------------- Fetching page {}", page_number);

        // Get the page from the Hypixel API
        let before_page_request = Instant::now();
        let page_request = get_auction_page(page_number).await;
        if page_request.is_none() {
            num_failed += 1;
            error(format!(
                "Failed to fetch page {} with a total of {} failed pages",
                page_number, num_failed
            ))
            .await;
            continue;
        }
        debug!(
            "Request time: {} ms",
            before_page_request.elapsed().as_millis()
        );

        // Parse the auctions and add them to the auctions array
        let before_page_parse = Instant::now();
        auctions.append(&mut parse_hypixel(page_request.unwrap().auctions));
        debug!(
            "Parsing time: {}ms",
            before_page_parse.elapsed().as_millis()
        );

        debug!(
            "Total time: {}ms",
            before_page_request.elapsed().as_millis()
        );
    }

    info(format!(
        "Total fetch time: {}s",
        started.elapsed().as_secs()
    ))
    .await;

    // Update the auctions in the database
    debug!("Inserting into database");
    unsafe {
        // Empty table
        let _ = DATABASE
            .as_ref()
            .unwrap()
            .simple_query("TRUNCATE TABLE query")
            .await;
        // Prepare copy statement
        let copy_statement = DATABASE
            .as_ref()
            .unwrap()
            .prepare("COPY query FROM STDIN BINARY")
            .await
            .unwrap();
        // Create a sink for the copy statement
        let copy_sink = DATABASE
            .as_ref()
            .unwrap()
            .copy_in(&copy_statement)
            .await
            .unwrap();
        // Write used to write to the copy sink
        let copy_writer = BinaryCopyInWriter::new(
            copy_sink,
            &[
                Type::TEXT,
                Type::TEXT,
                Type::INT8,
                Type::TEXT,
                Type::TEXT,
                Type::TEXT,
                Type::INT8,
                Type::TEXT_ARRAY,
            ],
        );
        pin_mut!(copy_writer);
        // Write to copy sink
        let mut row: Vec<&'_ (dyn ToSql + Sync)> = Vec::new();
        let mut inserted_uuids: Vec<String> = Vec::new();
        for m in &auctions {
            // Prevent duplicates because for some reason there are duplicates
            if !inserted_uuids.contains(&m.uuid) {
                inserted_uuids.push(m.uuid.to_string());
                row.clear();
                row.push(&m.uuid);
                row.push(&m.auctioneer);
                row.push(&m.end_t);
                row.push(&m.item_name);
                row.push(&m.tier);
                row.push(&m.item_id);
                row.push(&m.starting_bid);
                row.push(&m.enchants);
                copy_writer.as_mut().write(&row).await.unwrap();
            }
        }
        // Complete the copy statement
        let out = copy_writer.finish().await;

        match out {
            Ok(_) => {
                info("Successfully inserted into database".to_string()).await;
            }
            Err(e) => error(format!("Error inserting into database: {}", e)).await,
        }
    }

    info(format!(
        "Total fetch and insert time taken {}s",
        started.elapsed().as_secs()
    ))
    .await;

    *IS_UPDATING.lock().unwrap() = false;
    *TOTAL_UPDATES.lock().unwrap() += 1;
}

/* Gets an auction page from the Hypixel API */
pub async fn get_auction_page(page_number: i64) -> Option<AuctionResponse> {
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

    None
}

/* Parses a page of auctions to a vector of documents  */
pub fn parse_hypixel(auctions: Vec<Item>) -> Vec<DatabaseItem> {
    // Stores the parsed auctions
    let mut new_auctions: Vec<DatabaseItem> = Vec::new();

    for auction in auctions {
        /* Only bins (for now?) */
        if let Some(true) = auction.bin {
            // Parse the auction's nbt
            let nbt = &auction.to_nbt().unwrap().i[0];
            // Item id
            let id = nbt.tag.extra_attributes.id.clone();

            // Get enchants if the item is an enchanted book
            let mut enchants = Vec::new();
            if id == "ENCHANTED_BOOK" && nbt.tag.extra_attributes.enchantments.is_some() {
                for entry in nbt.tag.extra_attributes.enchantments.as_ref().unwrap() {
                    enchants.push(format!("{};{}", entry.0.to_uppercase(), entry.1));
                }
            }

            // Push this auctions to the array
            new_auctions.push(DatabaseItem {
                uuid: auction.uuid,
                auctioneer: auction.auctioneer,
                end_t: auction.end,
                item_name: if id != "ENCHANTED_BOOK" {
                    auction.item_name
                } else {
                    MC_CODE_REGEX
                        .replace_all(auction.item_lore.split("\n").next().unwrap_or(""), "")
                        .to_string()
                },
                tier: auction.tier,
                starting_bid: auction.starting_bid,
                item_id: id,
                enchants,
            });
        }
    }

    return new_auctions;
}
