# Rust query API
A versatile API facade for the Hypixel Auction API. The entire auction house is fetched and inserted into a PostgreSQL database with NBT parsing in under 12 seconds every minute! You can query and sort by auction uuid, auctioneer, end time, item name, item tier, item id, price, and enchants.

## Set up
### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
- [PostgresSQL database](https://www.postgresql.org/)
- [Discord](https://discord.com/)

### Steps
- Clone the repository
- Copy the `example_config.json` file into a new `config.json` file and fill all fields out
- Run `cargo run` with an optional `--release` flag for a much faster and more efficient program
- Use it!

### Configuration Fields
- `base_url`: The base url of the domain such as http://localhost:8080/
- `api_key`: Api key needed to access this api (NOT a Hypixel API key)
- `postgres_db_url`: Full url for the PostgreSQL database
- `webhook_url`: Discord webhook url for logging