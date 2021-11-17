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

use crate::webhook::Webhook;
use lazy_static::lazy_static;
use regex::Regex;
use std::sync::Mutex;
use tokio_postgres::Client;

lazy_static! {
    pub static ref HTTP_CLIENT: reqwest::Client = reqwest::Client::builder()
        .gzip(true)
        .brotli(true)
        .build()
        .unwrap();
    pub static ref MC_CODE_REGEX: Regex = Regex::new("(?i)\u{00A7}[0-9A-FK-OR]").unwrap();
    pub static ref BASE_URL: Mutex<String> = Mutex::new("".to_string());
    pub static ref PORT: Mutex<String> = Mutex::new("".to_string());
    pub static ref URL: Mutex<String> = Mutex::new("".to_string());
    pub static ref API_KEY: Mutex<String> = Mutex::new("".to_string());
    pub static ref POSTGRES_DB_URL: Mutex<String> = Mutex::new("".to_string());
    pub static ref IS_UPDATING: Mutex<bool> = Mutex::new(false);
    pub static ref TOTAL_UPDATES: Mutex<i16> = Mutex::new(0);
    pub static ref LAST_UPDATED: Mutex<i64> = Mutex::new(0);
    pub static ref ENABLE_QUERY: Mutex<bool> = Mutex::new(false);
    pub static ref ENABLE_PETS: Mutex<bool> = Mutex::new(false);
    pub static ref ENABLE_LOWESTBIN: Mutex<bool> = Mutex::new(false);
}

pub static mut DATABASE: Option<Client> = None;
pub static mut WEBHOOK: Option<Webhook> = None;
