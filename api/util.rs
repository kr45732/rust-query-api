use crate::nbt_utils::Item;
use crate::static_values::{HTTP_CLIENT, MC_CODE_REGEX};
use mongodb::bson::{doc, Document};
use serde::{Deserialize, Serialize};
use std::future::Future;
use tokio::time::{self, Duration};

#[derive(Serialize, Deserialize)]
pub struct AuctionResponse {
    #[serde(rename = "totalPages")]
    pub total_pages: i64,

    #[serde(rename = "auctions")]
    pub auctions: Vec<Item>,
}

pub async fn get(page: i64) -> AuctionResponse {
    let res = HTTP_CLIENT
        .get(format!(
            "https://api.hypixel.net/skyblock/auctions?page={}",
            page
        ))
        .send()
        .await
        .unwrap();
    let text = res.text().await.unwrap();
    serde_json::from_str(&text).unwrap()
}

pub fn parse_hypixel(auctions: Vec<Item>) -> Vec<Document> {
    let mut new_auctions: Vec<Document> = Vec::new();

    for auction in auctions {
        if let Some(true) = auction.bin {
            let nbt = &auction.to_nbt().unwrap().i[0];
            let id = nbt.tag.extra_attributes.id.clone();

            let mut enchants = Vec::new();
            if auction.item_name == "Enchanted Book"
                && nbt.tag.extra_attributes.enchantments.is_some()
            {
                for entry in nbt.tag.extra_attributes.enchantments.as_ref().unwrap() {
                    enchants.push(format!("{};{}", entry.0.to_uppercase(), entry.1));
                }
            }

            new_auctions.push(doc! {
                "uuid": auction.uuid,
                "auctioneer": auction.auctioneer,
                "end": auction.end,
                "item_name": if auction.item_name != "Enchanted Book" {
                    auction.item_name
                } else {
                    MC_CODE_REGEX
                        .replace_all(auction.item_lore.split("\n").next().unwrap_or(""), "")
                        .to_string()
                },
                "tier": auction.tier,
                "starting_bid": auction.starting_bid,
                "item_id": id,
                "enchants": enchants,
            });
        }
    }
    return new_auctions;
}

pub fn set_interval<F, Fut>(mut f: F, dur: Duration)
where
    F: Send + 'static + FnMut() -> Fut,
    Fut: Future<Output = ()> + Send + 'static,
{
    // Create stream of intervals.
    let mut interval = time::interval(dur);
    tokio::spawn(async move {
        // Skip the first tick at 0ms.
        interval.tick().await;
        loop {
            // Wait until next tick.
            interval.tick().await;
            // Spawn a task for this tick.
            tokio::spawn(f());
        }
    });
}
