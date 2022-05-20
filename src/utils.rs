use std::thread;

use ntex::http::Client;
use tokio::time::Duration;
use tracing::error;

async fn duration_until_update() -> Duration {
    let mut num_attempts = 0;
    let client = Client::default();
    loop {
        num_attempts += 1;
        let res = client
            .get("placeholder")
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
