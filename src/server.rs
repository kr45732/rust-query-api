use crate::fetch::{DATABASE, IS_UPDATING, LAST_UPDATED, TOTAL_UPDATES};
use crate::util::BASE_URL;
use futures::StreamExt;
use hyper::service::{make_service_fn, service_fn};
use hyper::{header, Body, Method, Request, Response, Server, StatusCode};
use log::{error, info};
use mongodb::bson::Document;
use mongodb::options::FindOptions;
use reqwest::Url;
use std::env;
use std::fmt::Write;
use std::io::Result;
use substring::Substring;

pub async fn start_server() {
    let vercel_url = env::var("VERCEL_URL");

    if vercel_url.is_ok() {
        let _ = BASE_URL.lock().unwrap().write_str(&vercel_url.unwrap());
    } else {
        let _ = BASE_URL.lock().unwrap().write_str("127.0.0.1:1337");
    }

    println!("{}", BASE_URL.lock().unwrap());

    let addr = BASE_URL.lock().unwrap().parse().unwrap();

    let make_service =
        make_service_fn(|_| async { Ok::<_, hyper::Error>(service_fn(response_examples)) });

    let server = Server::bind(&addr).serve(make_service);

    info!("Listening on http://{}", addr);
    println!("Listening on http://{}", addr);

    if let Err(e) = server.await {
        error!("Error when starting server: {}", e);
    }
}

async fn response_examples(req: Request<Body>) -> Result<Response<Body>> {
    info!("{} {}", req.method(), req.uri().path().substring(0, 30));

    if let (&Method::GET, "/") = (req.method(), req.uri().path()) {
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
        let mut query = "{}".to_string();
        let mut sort = "{}".to_string();

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
            }
        }

        let query_result: std::result::Result<Document, serde_json::Error> =
            serde_json::from_str(&query);
        let sort_result: std::result::Result<Document, serde_json::Error> =
            serde_json::from_str(&sort);

        if query_result.is_err() {
            return bad_request("Invalid query JSON");
        }
        if sort_result.is_err() {
            return bad_request("Invalid sort JSON");
        }

        let query_doc: Document = query_result.unwrap();
        let sort_doc: Document = sort_result.unwrap();

        let query_options = FindOptions::builder()
            .sort(sort_doc)
            .allow_disk_use(true)
            .build();

        unsafe {
            let database_ref = DATABASE.as_ref();
            if database_ref.is_none() {
                return internal_error("Database isn't connected");
            }

            let results_cursor = database_ref
                .unwrap()
                .collection::<Document>("rust-query")
                .find(query_doc, query_options)
                .await;

            if results_cursor.is_err() {
                return internal_error("Error when querying database");
            }

            let mut cursor = results_cursor.unwrap();
            let mut results_vec = vec![];
            while let Some(doc) = cursor.next().await {
                results_vec.push(doc.unwrap());
            }

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

fn not_found() -> Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{\"success\":false}"))
        .unwrap())
}

fn bad_request(reason: &str) -> Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::BAD_REQUEST)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            "{{\"success\":false,\"reason\":\"{}\"}}",
            reason
        )))
        .unwrap())
}

fn internal_error(reason: &str) -> Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            "{{\"success\":false,\"reason\":\"{}\"}}",
            reason
        )))
        .unwrap())
}
