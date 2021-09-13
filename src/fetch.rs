use crate::util::{get, parse_hypixel};
use log::{debug, info};
use mongodb::{bson::Document, Client, Database};
use std::env;
use std::time::Instant;

pub static mut DATABASE: Option<Database> = None;
pub static mut IS_UPDATING: bool = false;
pub static mut TOTAL_UPDATES: i16 = 0;
pub static mut LAST_UPDATED: i64 = 0;

pub async fn fetch_auctions() {
    info!("Fetching auctions");
    let started = Instant::now();
    unsafe {
        IS_UPDATING = true;
    }

    let mut auctions: Vec<Document> = Vec::new();

    let r = get(1).await;
    auctions.append(&mut parse_hypixel(r.auctions));
    for page_number in 2..3 {
        //r.total_pages {
        debug!("---------------- Fetching page {}", page_number);

        // Make request
        let now = Instant::now();
        let page_request = get(page_number).await;
        debug!("Request took {} ms", now.elapsed().as_millis());

        // Add auctions to array
        let nowss = Instant::now();
        auctions.append(&mut parse_hypixel(page_request.auctions));
        debug!("Parsing took {} ms", nowss.elapsed().as_millis());

        debug!("Total time is {} ms", now.elapsed().as_millis());
    }

    info!("Total fetch time taken {} ms", started.elapsed().as_secs());

    debug!("Inserting into database");
    unsafe {
        let collection = DATABASE
            .get_or_insert(
                Client::with_uri_str(env::var("MONGO_DB_URL").unwrap())
                    .await
                    .unwrap()
                    .database("skyblock"),
            )
            .collection::<Document>("rust-query");
        let _ = collection.drop_indexes(Option::None).await;
        let _ = collection.insert_many(auctions, Option::None).await;
    }
    debug!("Finished inserting into database");

    info!(
        "Total fetch and insert time taken {} ms",
        started.elapsed().as_secs()
    );
    unsafe {
        IS_UPDATING = false;
        TOTAL_UPDATES += 1;
    }
}
