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

use crate::utils::{is_false, median};
use dashmap::DashMap;
use postgres_types::{FromSql, ToSql};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;
use tokio_postgres::Row;

/* Query API */
#[derive(Serialize)]
pub struct QueryDatabaseItem {
    pub uuid: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub score: Option<i32>,
    pub auctioneer: String,
    pub end_t: i64,
    pub item_name: String,
    pub lore: String,
    pub tier: String,
    pub item_id: String,
    pub internal_id: String,
    pub starting_bid: i64,
    pub highest_bid: i64,
    pub bin: bool,
    pub count: i16,
    #[serde(skip_serializing)]
    pub lowestbin_price: f32,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub enchants: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub attributes: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub bids: Vec<Bid>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub potato_books: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stars: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub farming_for_dummies: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transmission_tuner: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub mana_disintegrator: Option<i16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reforge: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rune: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skin: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub power_scroll: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drill_upgrade_module: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drill_fuel_tank: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub drill_engine: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub dye: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub accessory_enrichment: Option<String>,
    #[serde(skip_serializing_if = "is_false")]
    pub recombobulated: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub wood_singularity: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub art_of_war: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub art_of_peace: bool,
    #[serde(skip_serializing_if = "is_false")]
    pub etherwarp: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub necron_scrolls: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gemstones: Option<Vec<String>>,
}

impl From<Row> for QueryDatabaseItem {
    fn from(row: Row) -> Self {
        Self {
            uuid: row.get("uuid"),
            score: row.try_get("score").unwrap_or(None),
            auctioneer: row.get("auctioneer"),
            end_t: row.get("end_t"),
            item_name: row.get("item_name"),
            lore: row.get("lore"),
            tier: row.get("tier"),
            item_id: row.get("item_id"),
            internal_id: row.get("internal_id"),
            starting_bid: row.get("starting_bid"),
            highest_bid: row.get("highest_bid"),
            lowestbin_price: row.get("lowestbin_price"),
            enchants: row.get("enchants"),
            attributes: row.get("attributes"),
            bin: row.get("bin"),
            bids: row.get("bids"),
            count: row.get("count"),
            potato_books: row.get("potato_books"),
            stars: row.get("stars"),
            farming_for_dummies: row.get("farming_for_dummies"),
            transmission_tuner: row.get("transmission_tuner"),
            mana_disintegrator: row.get("mana_disintegrator"),
            reforge: row.get("reforge"),
            rune: row.get("rune"),
            skin: row.get("skin"),
            power_scroll: row.get("power_scroll"),
            drill_upgrade_module: row.get("drill_upgrade_module"),
            drill_fuel_tank: row.get("drill_fuel_tank"),
            drill_engine: row.get("drill_engine"),
            dye: row.get("dye"),
            accessory_enrichment: row.get("accessory_enrichment"),
            recombobulated: row.get("recombobulated"),
            wood_singularity: row.get("wood_singularity"),
            art_of_war: row.get("art_of_war"),
            art_of_peace: row.get("art_of_peace"),
            etherwarp: row.get("etherwarp"),
            necron_scrolls: row.get("necron_scrolls"),
            gemstones: row.get("gemstones"),
        }
    }
}

#[derive(Debug, ToSql, FromSql, Deserialize, Serialize)]
#[postgres(name = "bid")]
pub struct Bid {
    pub bidder: String,
    pub amount: i64,
}

/* Average Auction API */
pub struct AverageDatabaseItem {
    pub item_id: String,
    pub prices: Vec<AvgAh>,
}

impl AverageDatabaseItem {
    pub fn get_sales(&self, count: f32) -> f32 {
        self.prices.iter().map(|e| e.sales).sum::<f32>() / count
    }

    pub fn get_average(&self) -> f32 {
        self.prices.iter().map(|e| e.price).sum::<f32>() / self.prices.len() as f32
    }

    pub fn get_median(&self) -> f32 {
        let combined_vec: Vec<f32> = self.prices.iter().map(|e| e.price).collect();
        median(&combined_vec)
    }

    pub fn get_modified_median(&self, percent: f32) -> f32 {
        let combined_vec: Vec<f32> = self.prices.iter().map(|e| e.price).collect();

        let median = median(&combined_vec);
        let lower_bound = median * (1.0 - percent);
        let upper_bound = median * (1.0 + percent);

        let mut sum = 0.0;
        let mut count = 0;

        for ele in combined_vec {
            if lower_bound <= ele && ele <= upper_bound {
                sum += ele;
                count += 1
            }
        }

        if count == 0 {
            median
        } else {
            sum / count as f32
        }
    }
}

impl From<Row> for AverageDatabaseItem {
    fn from(row: Row) -> Self {
        Self {
            item_id: row.get(0),
            prices: row.get(1),
        }
    }
}

#[derive(Debug, ToSql, FromSql)]
#[postgres(name = "avg_ah")]
pub struct AvgAh {
    pub price: f32,
    pub sales: f32,
}

#[derive(Serialize)]

pub struct PartialAvgAh {
    pub price: f32,
    pub sales: f32,
}

pub struct AvgSum {
    pub sum: i64,
    pub count: i32,
}

impl AvgSum {
    pub fn update(&mut self, sum: i64, count: i32) {
        self.sum += sum;
        self.count += count;
    }

    pub fn get_average(&self) -> i64 {
        self.sum / self.count as i64
    }
}

/* Pets API */
#[derive(Serialize)]
pub struct PetsDatabaseItem {
    pub name: String,
    pub price: i64,
}

impl From<Row> for PetsDatabaseItem {
    fn from(row: Row) -> Self {
        Self {
            name: row.get("name"),
            price: row.get("price"),
        }
    }
}

/* NBT */
#[derive(Deserialize)]
pub struct PartialNbt {
    pub i: Vec<PartialNbtElement>,
}

#[derive(Deserialize)]
pub struct PartialNbtElement {
    #[serde(rename = "Count")]
    pub count: i16,
    pub tag: PartialTag,
}

#[derive(Deserialize)]
pub struct PartialTag {
    #[serde(rename = "ExtraAttributes")]
    pub extra_attributes: PartialExtraAttr,
    pub display: DisplayInfo,
}

#[derive(Deserialize)]
pub struct PartialExtraAttr {
    pub id: String,
    #[serde(rename = "petInfo")]
    pub pet: Option<String>,
    pub enchantments: Option<DashMap<String, i32>>,
    pub runes: Option<DashMap<String, i32>>,
    pub attributes: Option<BTreeMap<String, i32>>,
    pub party_hat_color: Option<String>,
    pub party_hat_emoji: Option<String>,
    pub new_years_cake: Option<i32>,
    pub winning_bid: Option<i64>,
    pub hot_potato_count: Option<i16>,
    pub upgrade_level: Option<i16>,
    pub dungeon_item_level: Option<i16>,
    pub farming_for_dummies_count: Option<i16>,
    pub tuned_transmission: Option<i16>,
    pub mana_disintegrator_count: Option<i16>,
    pub modifier: Option<String>,
    pub skin: Option<String>,
    pub power_ability_scroll: Option<String>,
    pub drill_part_upgrade_module: Option<String>,
    pub drill_part_fuel_tank: Option<String>,
    pub drill_part_engine: Option<String>,
    pub dye_item: Option<String>,
    pub talisman_enrichment: Option<String>,
    pub rarity_upgrades: Option<i16>,
    pub wood_singularity_count: Option<i16>,
    pub art_of_war_count: Option<i16>,
    #[serde(rename = "artOfPeaceApplied")]
    pub art_of_peace_applied: Option<i16>,
    pub ethermerge: Option<i16>,
    pub ability_scroll: Option<Vec<String>>,
    pub gems: Option<DashMap<String, Value>>,
}

impl PartialExtraAttr {
    pub fn get_stars(&self) -> Option<i16> {
        if self.upgrade_level.is_some() {
            self.upgrade_level
        } else {
            self.dungeon_item_level
        }
    }

    pub fn get_rune(&self) -> Option<String> {
        if let Some(runes_val) = &self.runes {
            if let Some(ele) = runes_val.into_iter().next() {
                return Some(format!("{}_RUNE;{}", ele.key(), ele.value()));
            }
        }

        None
    }

    pub fn get_talisman_enrichment(&self) -> Option<String> {
        if let Some(talisman_enrichment_value) = &self.talisman_enrichment {
            return Some(format!("TALISMAN_ENRICHMENT_{}", talisman_enrichment_value));
        }

        None
    }

    pub fn is_recombobulated(&self) -> bool {
        if let Some(rarity_upgrades_value) = &self.rarity_upgrades {
            return rarity_upgrades_value == &1;
        }

        false
    }

    pub fn is_wood_singularity_applied(&self) -> bool {
        if let Some(wood_singularity_count_value) = &self.wood_singularity_count {
            return wood_singularity_count_value == &1;
        }

        false
    }

    pub fn is_art_of_war_applied(&self) -> bool {
        if let Some(art_of_war_count_value) = &self.art_of_war_count {
            return art_of_war_count_value == &1;
        }

        false
    }

    pub fn is_art_of_peace_applied(&self) -> bool {
        if let Some(art_of_peace_value) = &self.art_of_peace_applied {
            return art_of_peace_value == &1;
        }

        false
    }

    pub fn is_etherwarp_applied(&self) -> bool {
        if let Some(ethermerge_value) = &self.ethermerge {
            return ethermerge_value == &1;
        }

        false
    }

    pub fn get_gemstones(&self) -> Option<Vec<String>> {
        if let Some(gems_value) = &self.gems {
            // Slot includes number (e.g. COMBAT_0)
            // {SLOT}_{QUALITY}_{VARIETY}_GEM
            // AMBER_0_FINE_AMBER_GEM

            let mut out = Vec::new();
            for ele in gems_value {
                if !ele.key().ends_with("_gem") && ele.key() != "unlocked_slots" {
                    let quality;
                    if ele.value().is_string() {
                        quality = ele.value().as_str().unwrap();
                    } else if ele.value().is_object() {
                        quality = ele
                            .value()
                            .as_object()
                            .unwrap()
                            .get("quality")
                            .unwrap()
                            .as_str()
                            .unwrap();
                    } else {
                        continue;
                    }

                    let gem_key = format!("{}_gem", ele.key());
                    if let Some(gem) = gems_value.get(&gem_key) {
                        // "COMBAT_0": "PERFECT" & "COMBAT_0_gem": "JASPER"

                        out.push(format!(
                            "{}_{}_{}_GEM",
                            ele.key(),
                            quality,
                            gem.value().as_str().unwrap()
                        ));
                    } else {
                        // "RUBY_0": "PERFECT"
                        out.push(format!(
                            "{}_{}_{}_GEM",
                            ele.key(),
                            quality,
                            ele.key().split('_').next().unwrap()
                        ));
                    }
                }
            }

            if !out.is_empty() {
                return Some(out);
            }
        }

        None
    }
}

#[derive(Deserialize)]
pub struct DisplayInfo {
    #[serde(rename = "Name")]
    pub name: String,
}

#[derive(Deserialize)]
pub struct PetInfo {
    pub tier: String,
    #[serde(rename = "heldItem")]
    pub held_item: Option<String>,
}

#[derive(Deserialize)]
pub struct Auctions {
    pub page: i32,
    #[serde(rename = "totalPages")]
    pub total_pages: i32,
    #[serde(rename = "lastUpdated")]
    pub last_updated: i64,
    pub auctions: Vec<Auction>,
}

#[derive(Deserialize)]
pub struct Auction {
    pub uuid: String,
    pub auctioneer: String,
    pub end: i64,
    pub item_name: String,
    pub item_lore: String,
    pub tier: String,
    pub starting_bid: i64,
    pub highest_bid_amount: i64,
    pub item_bytes: String,
    pub bin: bool,
    pub bids: Vec<Bid>,
    pub last_updated: i64,
}

#[derive(Deserialize)]
pub struct EndedAuctions {
    #[serde(rename = "lastUpdated")]
    pub last_updated: i64,
    pub auctions: Vec<EndedAuction>,
}

#[derive(Deserialize)]
pub struct EndedAuction {
    pub price: i64,
    pub bin: bool,
    pub item_bytes: String,
    pub auction_id: String,
}
