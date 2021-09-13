use query_api::{fetch::fetch_auctions, server::start_server, util::set_interval};
use simplelog::*;
use std::fs::File;
use tokio::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Creating loggers");
    CombinedLogger::init(vec![
        WriteLogger::new(
            LevelFilter::Info,
            Config::default(),
            File::create("info.log").unwrap(),
        ),
        WriteLogger::new(
            LevelFilter::Debug,
            Config::default(),
            File::create("debug.log").unwrap(),
        ),
    ])
    .unwrap();
    println!("Loggers created");

    println!("Starting auction loop");
    fetch_auctions().await;

    set_interval(
        || async {
            fetch_auctions().await;
        },
        Duration::from_millis(300000),
    );

    start_server().await;

    Ok(())
}
