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

use dashmap::DashMap;
use postgres_types::{FromSql, ToSql};
use serde::{Deserialize, Serialize};
use tokio_postgres::Row;

#[derive(Debug, Deserialize, Serialize, ToSql, FromSql)]
pub struct DatabaseItem {
    pub uuid: String,
    pub auctioneer: String,
    pub end_t: i64,
    pub item_name: String,
    pub tier: String,
    pub item_id: String,
    pub starting_bid: i64,
    pub enchants: Vec<String>,
    pub bin: bool,
    pub bids: Vec<Bid>,
}

impl From<Row> for DatabaseItem {
    fn from(row: Row) -> Self {
        Self {
            uuid: row.get("uuid"),
            auctioneer: row.get("auctioneer"),
            end_t: row.get("end_t"),
            item_name: row.get("item_name"),
            tier: row.get("tier"),
            item_id: row.get("item_id"),
            starting_bid: row.get("starting_bid"),
            enchants: row.get("enchants"),
            bin: row.get("bin"),
            bids: row.get("bids"),
        }
    }
}

#[derive(Debug, ToSql, FromSql, Deserialize, Serialize)]
#[postgres(name = "bid")]
pub struct Bid {
    pub bidder: String,
    pub amount: i64,
}

#[derive(Debug, Deserialize, Serialize)]
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

#[derive(Deserialize)]
pub struct PartialNbt {
    pub i: Vec<PartialNbtElement>,
}

#[derive(Deserialize)]
pub struct PartialNbtElement {
    // #[serde(rename = "Count")]
    // pub count: i64,
    pub tag: PartialTag,
}

#[derive(Deserialize)]
pub struct PartialTag {
    #[serde(rename = "ExtraAttributes")]
    pub extra_attributes: PartialExtraAttr,
    // pub display: DisplayInfo,
}

#[derive(Deserialize)]
pub struct PartialExtraAttr {
    pub id: String,
    #[serde(rename = "petInfo")]
    pub pet: Option<String>,
    pub enchantments: Option<DashMap<String, i32>>,
}

// #[derive(Deserialize)]
// pub struct DisplayInfo {
//     #[serde(rename = "Name")]
//     pub name: String,
//     #[serde(rename = "Lore")]
//     pub lore: Vec<String>,
// }

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum ItemBytes {
    T0(ItemBytesT0),
    Data(String),
}

impl Into<String> for ItemBytes {
    fn into(self) -> String {
        match self {
            Self::T0(ibt0) => {
                let ItemBytesT0::Data(x) = ibt0;
                x
            }
            Self::Data(x) => x,
        }
    }
}

impl Into<Result<Vec<u8>, Box<dyn std::error::Error>>> for ItemBytes {
    fn into(self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let b64: String = self.into();
        Ok(base64::decode(&b64)?)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(tag = "type", content = "data")]
pub enum ItemBytesT0 {
    #[serde(rename = "0")]
    Data(String),
}
