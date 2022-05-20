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

use enumset::{EnumSet, EnumSetType};
use std::env;
use std::str::FromStr;

#[derive(Debug, EnumSetType)]
pub enum Feature {
    Query,
    Pets,
    Lowestbin,
    Underbin,
    AverageAuction,
}

pub struct Config {
    pub enabled_features: EnumSet<Feature>,
    pub port: u16,
    pub postgres_url: String,
    pub api_key: String,
    pub admin_api_key: String,
}

impl FromStr for Feature {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "QUERY" => Self::Query,
            "PETS" => Self::Pets,
            "LOWESTBIN" => Self::Lowestbin,
            "UNDERBIN" => Self::Underbin,
            "AVERAGE_AUCTION" => Self::AverageAuction,
            _ => return Err(format!("Unknown feature flag: \"{}\"", s)),
        })
    }
}

fn get_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("Unable to find \"{}\" environment variable", name))
}

impl Config {
    pub fn load() -> Self {
        let port = get_env("PORT").parse::<u16>().expect("Invalid PORT");
        let api_key = get_env("API_KEY");
        let admin_api_key = env::var("ADMIN_API_KEY").unwrap_or_else(|_| api_key.clone());
        let postgres_url = get_env("POSTGRES_URL");
        let features = get_env("FEATURES")
            .split('+')
            .map(|s| Feature::from_str(s).unwrap())
            .fold(EnumSet::<Feature>::new(), |x, y| x | y);
        if features.contains(Feature::Underbin) && !features.contains(Feature::Lowestbin) {
            panic!("Please enable LOWESTBIN if you want to enable UNDERBIN");
        }
        Config {
            enabled_features: features,
            postgres_url,
            api_key,
            admin_api_key,
            port,
        }
    }
}
