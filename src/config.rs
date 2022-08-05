use std::collections::HashSet;
use std::env;
use std::str::FromStr;

#[derive(Debug, PartialEq, Eq, Hash)]
pub enum Feature {
    Query,
    Pets,
    Lowestbin,
    Underbin,
    AverageAuction,
    AverageBin,
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
            "AVERAGE_BIN" => Self::AverageBin,
            _ => return Err(format!("Unknown feature flag {}", s)),
        })
    }
}

pub struct Config {
    pub enabled_features: HashSet<Feature>,
    pub webhook_url: String,
    pub base_url: String,
    pub port: u32,
    pub full_url: String,
    pub postgres_url: String,
    pub api_key: String,
    pub admin_api_key: String,
    pub debug: bool,
}

fn get_env(name: &str) -> String {
    env::var(name).unwrap_or_else(|_| panic!("Unable to find {} environment variable", name))
}

impl Config {
    pub fn load_or_panic() -> Self {
        let base_url = get_env("BASE_URL");
        let port = get_env("PORT").parse::<u32>().expect("PORT not valid");
        let api_key = get_env("API_KEY");
        let webhook_url = env::var("WEBHOOK_URL").unwrap_or_default();
        let admin_api_key = env::var("ADMIN_API_KEY").unwrap_or_else(|_| api_key.clone());
        let debug = env::var("DEBUG")
            .unwrap_or_else(|_| String::from("false"))
            .parse()
            .unwrap_or(false);
        let postgres_url = get_env("POSTGRES_URL");
        let features = get_env("FEATURES")
            .replace(',', "+")
            .split('+')
            .map(|s| Feature::from_str(s).unwrap())
            .collect::<HashSet<Feature>>();
        if features.contains(&Feature::Underbin) && !features.contains(&Feature::Lowestbin) {
            panic!("The LOWESTBIN feature must be enabled to enable the UNDERBIN feature");
        }
        Config {
            enabled_features: features,
            full_url: format!("{}:{}", base_url, port),
            postgres_url,
            base_url,
            webhook_url,
            api_key,
            admin_api_key,
            port,
            debug,
        }
    }

    pub fn is_enabled(&self, feature: Feature) -> bool {
        self.enabled_features.contains(&feature)
    }
}
