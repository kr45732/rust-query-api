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
- Copy the `example_env` file into a new `.env` file and fill all fields out OR set all the fields using environment variables
- Run `cargo run --release` (this may take some time to compile)
- Use it!

### Configuration Fields or Environment Variables
- `BASE_URL`: The base url of the domain such as localhost
- `PORT`: The port such as 8080
- `API_KEY`: Key needed to access this api (NOT a Hypixel API key)
- `POSTGRES_URL`: Full url of a PostgreSQL database
- `WEBHOOK_URL`: Discord webhook url for logging

## Endpoints
- `/query?key=key&query=query&sort=sort`
- `/pets?key=key&query=query`

## Todo
- Improved error handling
- Lowest bin prices
- Prevent SQL injection
- Better documentation 