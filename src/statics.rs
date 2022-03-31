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
use deadpool_postgres::Pool;
use lazy_static::lazy_static;
use postgres_types::Type;
use regex::Regex;
use std::time::Duration;
use tokio::sync::Mutex;

lazy_static! {
    pub static ref HTTP_CLIENT: surf::Client = surf::Config::new()
        .set_timeout(Some(Duration::from_secs(15)))
        .try_into()
        .unwrap();
    pub static ref MC_CODE_REGEX: Regex = Regex::new("(?i)\u{00A7}[0-9A-FK-OR]").unwrap();
    pub static ref BASE_URL: Mutex<String> = Mutex::new("".to_string());
    pub static ref PORT: Mutex<String> = Mutex::new("".to_string());
    pub static ref URL: Mutex<String> = Mutex::new("".to_string());
    pub static ref API_KEY: Mutex<String> = Mutex::new("".to_string());
    pub static ref ADMIN_API_KEY: Mutex<String> = Mutex::new("".to_string());
    pub static ref POSTGRES_DB_URL: Mutex<String> = Mutex::new("".to_string());
    pub static ref IS_UPDATING: Mutex<bool> = Mutex::new(false);
    pub static ref TOTAL_UPDATES: Mutex<i16> = Mutex::new(0);
    pub static ref LAST_UPDATED: Mutex<i64> = Mutex::new(0);
    pub static ref ENABLE_QUERY: Mutex<bool> = Mutex::new(false);
    pub static ref ENABLE_PETS: Mutex<bool> = Mutex::new(false);
    pub static ref ENABLE_LOWESTBIN: Mutex<bool> = Mutex::new(false);
    pub static ref ENABLE_UNDERBIN: Mutex<bool> = Mutex::new(false);
    pub static ref ENABLE_AVERAGE_AUCTION: Mutex<bool> = Mutex::new(false);
    pub static ref WEBHOOK: Mutex<Option<Webhook>> = Mutex::new(None);
    pub static ref BID_ARRAY: Mutex<Option<Type>> = Mutex::new(None);
    pub static ref DATABASE: Mutex<Option<Pool>> = Mutex::new(None);
}
