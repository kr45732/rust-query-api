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

use crate::statics::WEBHOOK;
use chrono::prelude::{DateTime, Utc};
use futures::Future;
use hyper::{header, Body, Response, StatusCode};
use log::{error, info};
use std::time::SystemTime;
use tokio::time::{self, Duration};

/* 404 */
pub fn not_found() -> hyper::Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::NOT_FOUND)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from("{\"success\":false}"))
        .unwrap())
}

/* 400 */
pub fn bad_request(reason: &str) -> hyper::Result<Response<Body>> {
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
pub fn internal_error(reason: &str) -> hyper::Result<Response<Body>> {
    Ok(Response::builder()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .header(header::CONTENT_TYPE, "application/json")
        .body(Body::from(format!(
            "{{\"success\":false,\"reason\":\"{}\"}}",
            reason
        )))
        .unwrap())
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

pub async fn info(desc: String) {
    info!("{}", desc);
    unsafe {
        let _ = WEBHOOK
            .as_ref()
            .unwrap()
            .send(|message| {
                message.embed(|embed| {
                    embed
                        .title("Information")
                        .color(0x00FFFF)
                        .description(&desc)
                        .timestamp(&get_discord_timestamp())
                })
            })
            .await;
    }
}

pub async fn error(desc: String) {
    error!("{}", desc);
    unsafe {
        let _ = WEBHOOK
            .as_ref()
            .unwrap()
            .send(|message| {
                message.embed(|embed| {
                    embed
                        .title("Error")
                        .color(0xFF0000)
                        .description(&desc)
                        .timestamp(&get_discord_timestamp())
                })
            })
            .await;
    }
}

fn get_discord_timestamp() -> String {
    let dt: DateTime<Utc> = SystemTime::now().into();
    format!("{}", dt.format("%+"))
}
