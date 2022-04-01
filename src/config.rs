use std::env;
use std::str::FromStr;

use enumset::{EnumSet, EnumSetType};

#[derive(Debug, EnumSetType)]
pub enum Feature {
    QUERY,
    PETS,
    LOWESTBIN,
    UNDERBIN,
    AVERAGE_AUCTION,
}

impl FromStr for Feature {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(match s {
            "QUERY" => Self::QUERY,
            "PETS" => Self::PETS,
            "LOWESTBIN" => Self::LOWESTBIN,
            "UNDERBIN" => Self::UNDERBIN,
            "AVERAGE_AUCTION" => Self::AVERAGE_AUCTION,
            _ => return Err(format!("Unknown feature flag {}", s))
        })
    }
}

pub struct Config {
    pub enabled_features: EnumSet<Feature>,
    pub webhook_url: String,
    pub base_url: String,
    pub port: u32,
    pub full_url: String,
    pub postgres_url: String,
    pub api_key: String,
    pub admin_api_key: String,
}

fn get_env(name: &str) -> String {
    env::var(name).expect(&format!("Unable to find {} environment variable", name))
}


impl Config {
    pub fn load_or_panic() -> Self {
        let base_url = get_env("BASE_URL");
        let port = get_env("PORT").parse::<u32>().expect("PORT not valid");
        let api_key = get_env("API_KEY");
        let webhook_url = get_env("WEBHOOK_URL");
        let admin_api_key = env::var("ADMIN_API_KEY").unwrap_or(api_key.clone());
        let postgres_url = get_env("POSTGRES_URL");
        let features = get_env("FEATURES")
            .split("+")
            .map(|s| Feature::from_str(s).unwrap())
            .fold(EnumSet::<Feature>::new(), |x, y| x | y);
        if features.contains(Feature::UNDERBIN) && !features.contains(Feature::LOWESTBIN) {
            panic!("Please enable LOWESTBIN if you want to enable UNDERBIN");
        }
        Config {
            enabled_features: features,
            full_url: format!("{}:{}", base_url, port),
            postgres_url: postgres_url,
            base_url: base_url,
            webhook_url: webhook_url,
            api_key: api_key,
            admin_api_key: admin_api_key,
            port,
        }
    }
}


