use std::thread;

// use futures::Future;
use ntex::http::Client;
use tokio::time::Duration;
use tracing::error;

#[allow(dead_code)]
// + Send + Sync
pub async fn duration_until_update() -> Duration {
    let mut num_attempts = 0;
    let client = Client::default();
    loop {
        num_attempts += 1;
        let res = client
            .get("https://api.hypixel.net/skyblock/auctions?page=0")
            .header("User-Agent", "ntex::web")
            .send()
            .await;

        match res {
            Ok(res) => match res.header("age") {
                Some(age_header) => {
                    let age: u64 = age_header
                        .to_str()
                        .unwrap_or_default()
                        .parse::<u64>()
                        .unwrap();

                    // Cloudfare doesn't return an exact time in ms, so the +2 accounts for that
                    let time = 60 - age + 2;

                    // Retry in 15 seconds if headers are giving weird values
                    if time > 120 {
                        thread::sleep(Duration::from_secs(15));
                        continue;
                    }

                    // Cannot return 0 duration
                    if time == 0 {
                        return Duration::from_millis(1);
                    }

                    return Duration::from_secs(time);
                }
                None => return Duration::from_millis(1),
            },
            Err(_) => {
                thread::sleep(Duration::from_secs(15));
            }
        }
        if num_attempts == 15 {
            error!("Failed 15 consecutive attempts to contact the Hypixel API");
        }
    }
}

// pub async fn start_auction_loop<F, Fut>(mut f: F)
// where
//     F: Send + 'static + FnMut() -> Fut,
//     Fut: Future<Output = ()> + Send + Sync +  'static,
// {
//     // Create stream of intervals.
//     let mut interval = time::interval(duration_until_update().await);
//     tokio::task::Builder::new()
//     .name("auction_loop")
//     .spawn(async move {
//         loop {
//             // Skip tick at 0ms
//             interval.tick().await;
//             // Wait until next tick.
//             interval.tick().await;
//             // Spawn a task for this tick.
//             f().await;
//             // Updated to new interval
//             interval = time::interval(duration_until_update().await);
//         }
//     });
// }
