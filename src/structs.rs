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

use dashmap::DashMap;
use postgres_types::{FromSql, ToSql};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

/* Query API */
#[derive(Serialize)]
pub struct QueryDatabaseItem {
    pub uuid: String,
    pub auctioneer: String,
    pub end_t: i64,
    pub item_name: String,
    pub tier: String,
    pub item_id: String,
    pub internal_id: String,
    pub starting_bid: i64,
    #[serde(skip_serializing)]
    pub lowestbin_price: f64,
    pub enchants: Vec<String>,
    pub bin: bool,
    pub bids: Vec<Bid>,
    pub count: i32,
}

impl From<Row> for QueryDatabaseItem {
    fn from(row: Row) -> Self {
        Self {
            uuid: row.get("uuid"),
            auctioneer: row.get("auctioneer"),
            end_t: row.get("end_t"),
            item_name: row.get("item_name"),
            tier: row.get("tier"),
            item_id: row.get("item_id"),
            internal_id: row.get("internal_id"),
            starting_bid: row.get("starting_bid"),
            lowestbin_price: row.get("lowestbin_price"),
            enchants: row.get("enchants"),
            bin: row.get("bin"),
            bids: row.get("bids"),
            count: row.get("count"),
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
    pub time_t: i64,
    pub prices: Vec<AvgAh>,
}

impl From<Row> for AverageDatabaseItem {
    fn from(row: Row) -> Self {
        Self {
            time_t: row.get("time_t"),
            prices: row.get("prices"),
        }
    }
}

#[derive(Debug, ToSql, FromSql)]
#[postgres(name = "avg_ah")]
pub struct AvgAh {
    pub item_id: String,
    pub price: f64,
    pub sales: f32,
}

#[derive(Serialize)]

pub struct PartialAvgAh {
    pub price: f64,
    pub sales: f32,
}

pub struct AvgSum {
    pub sum: i64,
    pub count: i32,
}

impl AvgSum {
    pub fn add(self, new_amount: i64) -> Self {
        self.add_multiple(new_amount, 1)
    }

    pub fn add_multiple(mut self, sum: i64, count: i32) -> Self {
        self.sum += sum;
        self.count += count;
        self
    }

    pub fn get_average(&self) -> i64 {
        self.sum / self.count as i64
    }
}

pub struct AvgVec {
    pub sum: Vec<f64>,
    pub sales: Vec<f32>,
}

impl AvgVec {
    pub fn add(mut self, avg_ah: &AvgAh) -> Self {
        self.sum.push(avg_ah.price);
        self.sales.push(avg_ah.sales);
        self
    }

    pub fn from(avg_ah: &AvgAh) -> Self {
        Self {
            sum: vec![avg_ah.price],
            sales: vec![avg_ah.sales],
        }
    }

    pub fn get_average(&self) -> f64 {
        self.sum.iter().sum::<f64>() / (self.sum.len() as f64)
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
    pub count: i32,
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
    pub attributes: Option<DashMap<String, i32>>,
    pub party_hat_color: Option<String>,
    pub new_years_cake: Option<i32>,
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
    pub page: i64,
    #[serde(rename = "totalPages")]
    pub total_pages: i64,
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
    pub auctions: Vec<EndedAuction>,
}

#[derive(Deserialize)]
pub struct EndedAuction {
    pub price: i64,
    pub bin: bool,
    pub item_bytes: String,
    pub auction_id: String,
}
