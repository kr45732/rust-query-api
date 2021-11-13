# Rust query API
A versatile API facade for the Hypixel Auction API. The entire auction house is fetched with NBT parsing in under [insert benchmark] seconds every minute! You can query and sort by auction uuid, auctioneer, end time, item name, item tier, item id, price, and enchants.

## Set up
### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
- [PostgresSQL database](https://www.postgresql.org/)
### Steps
- Clone the repository
- Copy the `example_config.json` file into a new `config.json` file and fill all fields out
- Run `cargo run` with an optional `--release` flag for a much faster and more efficient program
- Use it!