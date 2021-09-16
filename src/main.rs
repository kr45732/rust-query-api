use futures::{Future, StreamExt};
use hyper::{header, Method, StatusCode};
use hyper::{
    service::{make_service_fn, service_fn},
    Body, Request, Response, Server,
};
use lazy_static::lazy_static;
use log::{debug, error, info};
use mongodb::bson::doc;
use mongodb::options::{FindOptions, InsertManyOptions};
use mongodb::{bson::Document, Client, Database};
use regex::Regex;
use reqwest::Url;
use serde::{Deserialize, Serialize};
use simplelog::*;
use std::collections::HashMap;
use std::result::Result as StdResult;
use std::time::Instant;
use std::{fmt::Write, fs::File, sync::Mutex};
use substring::Substring;
use tokio::time::{self, Duration};

lazy_static! {
    static ref HTTP_CLIENT: reqwest::Client = reqwest::Client::builder()
        .gzip(true)
        .brotli(true)
        .build()
        .unwrap();
    static ref MC_CODE_REGEX: Regex = Regex::new("(?i)\u{00A7}[0-9A-FK-OR]").unwrap();
    static ref BASE_URL: Mutex<String> = Mutex::new("".to_string());
    static ref API_KEY: Mutex<String> = Mutex::new("".to_string());
    static ref MONGO_DB_URL: Mutex<String> = Mutex::new("".to_string());
}

static mut DATABASE: Option<Database> = None;
static mut IS_UPDATING: bool = false;
static mut TOTAL_UPDATES: i16 = 0;
static mut LAST_UPDATED: i64 = 0;

/* Entry point to the program. Creates loggers, reads config, starts auction loop and server.  */
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Create log files
    println!("Creating log files...");
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
    println!("Loggers created.");

    // Read config
    println!("Reading config");
    let config: serde_json::Value =
        serde_json::from_reader(File::open("config.json").unwrap()).unwrap();
    let _ = BASE_URL
        .lock()
        .unwrap()
        .write_str(config.get("base_url").unwrap().as_str().unwrap());
    let _ = API_KEY
        .lock()
        .unwrap()
        .write_str(config.get("api_key").unwrap().as_str().unwrap());
    let _ = MONGO_DB_URL
        .lock()
        .unwrap()
        .write_str(config.get("mongo_db_url").unwrap().as_str().unwrap());

    // Start the auction loop
    println!("Starting auction loop...");
    fetch_auctions().await;

    set_interval(
        || async {
            fetch_auctions().await;
        },
        Duration::from_millis(150000),
    );

    // Start the server
    println!("Starting server...");
    start_server().await;

    Ok(())
}

/* Starts the server listening on BASE_URL */
pub async fn start_server() {
    let server_address = BASE_URL.lock().unwrap().parse().unwrap();

    let make_service =
        make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(response_examples)) });

    let server = Server::bind(&server_address).serve(make_service);

    println!("Listening on http://{}", server_address);

    if let Err(e) = server.await {
        error!("Error when starting server: {}", e);
    }
}

/* Handles http requests to the server */
async fn response_examples(req: Request<Body>) -> hyper::Result<Response<Body>> {
    info!("{} {}", req.method(), req.uri().path().substring(0, 30));

    if let (&Method::GET, "/") = (req.method(), req.uri().path()) {
        // Returns information & statistics about the API
        unsafe {
            Ok(Response::builder()
            .status(StatusCode::OK)
            .header(header::CONTENT_TYPE, "application/json")
            .body(Body::from(format!(
                "{{
                    \"success\":true,
                    \"information\":\"A versatile API facade for the Hypixel Auction API. Lets you query and sort by item id, name, and much more! This updates about every 1 minute. This API is currently private and is created by CrypticPlasma.\",
                    \"statistics\":
                    {{
                        \"is_updating\":\"{}\",
                        \"total_updates\":\"{}\",
                        \"last_updated\":\"{}\"
                    }}
                }}",
                IS_UPDATING, TOTAL_UPDATES, LAST_UPDATED
            )))
            .unwrap())
        }
    } else if let (&Method::GET, "/query") = (req.method(), req.uri().path()) {
        // Query paremeters
        let mut query = "{}".to_string();
        let mut sort = "{}".to_string();
        let mut key = "".to_string();

        // Reads the query parameters from the request and stores them in the corresponding variable
        for query_pair in Url::parse(
            &format!(
                "http://{}{}",
                BASE_URL.lock().unwrap(),
                &req.uri().to_string()
            )
            .to_string(),
        )
        .unwrap()
        .query_pairs()
        {
            if query_pair.0 == "query" {
                query = query_pair.1.to_string();
            } else if query_pair.0 == "sort" {
                sort = query_pair.1.to_string();
            } else if query_pair.0 == "key" {
                key = query_pair.1.to_string();
            }
        }

        // The API key in request doesn't match
        if key != API_KEY.lock().unwrap().as_str() {
            return bad_request("Not authorized");
        }

        if query == "{}" {
            return bad_request("The query JSON cannot be empty");
        }

        // Parse the query and sort JSONs
        let query_result: std::result::Result<Document, serde_json::Error> =
            serde_json::from_str(&query);
        let sort_result: std::result::Result<Document, serde_json::Error> =
            serde_json::from_str(&sort);

        // Invalid query JSONs
        if query_result.is_err() {
            return bad_request("Invalid query JSON");
        }
        // Invalid sort JOSN
        if sort_result.is_err() {
            return bad_request("Invalid sort JSON");
        }

        // Unwrap them to a Document
        let query_doc: Document = query_result.unwrap();
        let sort_doc: Document = sort_result.unwrap();

        unsafe {
            // Reference to the database
            let database_ref = DATABASE.as_ref();

            // Database isn't connected
            if database_ref.is_none() {
                return internal_error("Database isn't connected");
            }

            // Find and sort using query JSON
            let results_cursor = database_ref
                .unwrap()
                .collection::<Document>("rust-query")
                .find(
                    query_doc,
                    FindOptions::builder()
                        .sort(sort_doc)
                        .allow_disk_use(true)
                        .build(),
                )
                .await;

            // This shouldn't happen
            if results_cursor.is_err() {
                return internal_error("Error when querying database");
            }

            // Convert the cursor itterator to a vector
            let mut cursor = results_cursor.unwrap();
            let mut results_vec = vec![];
            while let Some(doc) = cursor.next().await {
                results_vec.push(doc.unwrap());
            }

            // Return the vector of auctions serialized into JSON
            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "application/json")
                .body(Body::from(serde_json::to_vec(&results_vec).unwrap()))
                .unwrap())
        }
    } else {
        not_found()
    }
}

/* 404 */
fn not_found() -> hyper::Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{\"success\":false}"))
        .unwrap())
}

/* 400 */
fn bad_request(reason: &str) -> hyper::Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            "{{\"success\":false,\"reason\":\"{}\"}}",
            reason
        )))
        .unwrap())
}

/* 500 */
fn internal_error(reason: &str) -> hyper::Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            "{{\"success\":false,\"reason\":\"{}\"}}",
            reason
        )))
        .unwrap())
}

/* Gets all pages of auctions from the Hypixel API and inserts them into the database */
pub async fn fetch_auctions() {
    info!("Fetching auctions...");

    let started = Instant::now();
    unsafe {
        IS_UPDATING = true;
    }

    // Stores all the auctions
    let mut auctions: Vec<Document> = Vec::new();

    // First page to get the total number of pages
    let r = get_auction_page(1).await;
    auctions.append(&mut parse_hypixel(r.auctions));
    for page_number in 2..r.total_pages {
        debug!("---------------- Fetching page {}", page_number);

        // Get the page from the Hypixel API
        let before_page_request = Instant::now();
        let page_request = get_auction_page(page_number).await;
        debug!(
            "Request took {} ms",
            before_page_request.elapsed().as_millis()
        );

        // Parse the auctions and add them to the auctions array
        let before_page_parse = Instant::now();
        auctions.append(&mut parse_hypixel(page_request.auctions));
        debug!(
            "Parsing time: {} ms",
            before_page_parse.elapsed().as_millis()
        );

        debug!(
            "Total time: {} ms",
            before_page_request.elapsed().as_millis()
        );
    }

    info!(
        "Total fetch time taken: {} seconds",
        started.elapsed().as_secs()
    );

    // Update the auctions in the database
    debug!("Inserting into database");
    unsafe {
        let mongo_url = MONGO_DB_URL.lock().unwrap().to_string();

        let collection = DATABASE
            .get_or_insert(
                Client::with_uri_str(mongo_url)
                    .await
                    .unwrap()
                    .database("skyblock"),
            )
            .collection::<Document>("rust-query");
        // Drop the collection to empty it
        let _ = collection.drop(Option::None).await;
        // Insert all the new auctions into the collection
        let _ = collection
            .insert_many(
                auctions,
                InsertManyOptions::builder().ordered(false).build(),
            )
            .await;
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

/* Gets an auction page from the Hypixel API */
pub async fn get_auction_page(page_number: i64) -> AuctionResponse {
    let res = HTTP_CLIENT
        .get(format!(
            "https://api.hypixel.net/skyblock/auctions?page={}",
            page_number
        ))
        .send()
        .await
        .unwrap();
    let text = res.text().await.unwrap();
    serde_json::from_str(&text).unwrap()
}

/* Parses a page of auctions to a vector of documents  */
pub fn parse_hypixel(auctions: Vec<Item>) -> Vec<Document> {
    // Stores the parsed auctions
    let mut new_auctions: Vec<Document> = Vec::new();

    for auction in auctions {
        /* Only bins (for now?) */
        if let Some(true) = auction.bin {
            // Parse the auction's nbt
            let nbt = &auction.to_nbt().unwrap().i[0];
            // Item id
            let id = nbt.tag.extra_attributes.id.clone();

            // Get enchants if the item is an enchanted book
            let mut enchants = Vec::new();
            if id == "ENCHANTED_BOOK" && nbt.tag.extra_attributes.enchantments.is_some() {
                for entry in nbt.tag.extra_attributes.enchantments.as_ref().unwrap() {
                    enchants.push(format!("{};{}", entry.0.to_uppercase(), entry.1));
                }
            }

            // Push this auctions to the array
            new_auctions.push(doc! {
                "uuid": auction.uuid,
                "auctioneer": auction.auctioneer,
                "end": auction.end,
                "item_name": if id != "ENCHANTED_BOOK" {
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

/* Repeat a task */
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

            f().await;
        }
    });
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
    pub display: DisplayInfo,
}

#[derive(Serialize, Deserialize)]
pub struct Pet {
    #[serde(rename = "type")]
    pub pet_type: String,

    #[serde(rename = "tier")]
    pub tier: String,
}

#[derive(Deserialize)]
pub struct PartialExtraAttr {
    pub id: String,
    // #[serde(rename = "petInfo")]
    // pub pet: Option<String>,
    pub enchantments: Option<HashMap<String, i32>>,
    // pub potion: Option<String>,
    // pub potion_level: Option<i16>,
    // pub anvil_uses: Option<i16>,
    // pub enhanced: Option<bool>,
    // pub runes: Option<HashMap<String, i32>>,
}

#[derive(Deserialize)]
pub struct DisplayInfo {
    #[serde(rename = "Name")]
    pub name: String,
    // #[serde(rename = "Lore")]
    // pub lore: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Item {
    #[serde(rename = "item_name")]
    pub item_name: String,
    #[serde(rename = "item_lore")]
    pub item_lore: String,
    #[serde(rename = "uuid")]
    pub uuid: String,
    #[serde(rename = "auctioneer")]
    pub auctioneer: String,
    #[serde(rename = "end")]
    pub end: i64,
    #[serde(rename = "tier")]
    pub tier: String,
    #[serde(rename = "item_bytes")]
    pub item_bytes: ItemBytes,
    #[serde(rename = "starting_bid")]
    pub starting_bid: i64,
    #[serde(rename = "bin")]
    pub bin: Option<bool>,
}

impl Item {
    pub fn to_nbt(&self) -> Result<PartialNbt, Box<dyn std::error::Error>> {
        let bytes: StdResult<Vec<u8>, _> = self.item_bytes.clone().into();
        let nbt: PartialNbt = nbt::from_gzip_reader(std::io::Cursor::new(bytes?))?;
        Ok(nbt)
    }
}

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

#[derive(Serialize, Deserialize)]
pub struct AuctionResponse {
    #[serde(rename = "totalPages")]
    pub total_pages: i64,

    #[serde(rename = "auctions")]
    pub auctions: Vec<Item>,
}
