# Rust Query API
<a href="https://github.com/kr45732/rust-query-api/releases" target="_blank">
  <img alt="downloads" src="https://img.shields.io/github/v/release/kr45732/rust-query-api?style=flat-square" />
</a>
<a href="https://github.com/kr45732/rust-query-api/blob/main/LICENSE" target="_blank">
  <img alt="license" src="https://img.shields.io/github/license/kr45732/rust-query-api?style=flat-square" />
</a>
<a href="https://dsc.gg/skyblock-plus" target="_blank">
  <img alt="license" src="https://img.shields.io/discord/796790757947867156?color=4166f5&label=discord&style=flat-square" />
</a> 

A versatile API facade for the Hypixel Auction API written in Rust. The entire auction house is fetched with NBT parsing and inserted into a PostgreSQL database in about 3-7 seconds every minute with low memory usage (can vary depending on enabled features, network speed, and latency of the Hypixel API)! You can query by auction UUID, auctioneer, end time, item name, item tier, item id, price, enchants, bin and bids. You can sort by the item's bin / starting price. You can track the last known price of each unique pet-level-rarity combination. You can track the lowest prices of all bins. It also can track new bins that are at least one million lower than previous bins. Lastly, it can track the average auction prices and sales up to five days with custom 'averaging methods'.

## Set Up
### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
- [PostgreSQL database](https://www.postgresql.org/)
- [Discord](https://discord.com/)

### Steps
- Clone the repository
- Copy the `example_env` file into a new `.env` file and fill all fields out **OR** set all the fields using environment variables
- Run `cargo run --release` (this may take some time to compile)
- Use it!

### Configuration Fields or Environment Variables
- `BASE_URL`: The base URL of the domain such as localhost
- `PORT`: The port such as 8080
- `API_KEY`: Key needed to access this API (NOT a Hypixel API key)
- `ADMIN_API_KEY`: Admin key required to use raw SQL parameters. Will default to the API_KEY if not provided
- `POSTGRES_URL`: Full URL of a PostgreSQL database
- `WEBHOOK_URL`: Discord webhook URL for logging
- `FEATURES`: The features (QUERY, PETS, LOWESTBIN, UNDERBIN, AVERAGE_AUCTION) you want to be enabled separated with a '+' 

## Usage
### Endpoints
- `/query`
- `/pets`
- `/lowestbin`
- `/underbin`
- `/average_auction`

### Docs & Examples
- See docs and examples [here](https://github.com/kr45732/rust-query-api/blob/main/examples/examples.md)

### Deploy To Railway
[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/new/template?template=https%3A%2F%2Fgithub.com%2Fkr45732%2Frust-query-api&plugins=postgresql&envs=BASE_URL%2CPORT%2CAPI_KEY%2CPOSTGRES_URL%2CWEBHOOK_URL%2CFEATURES&BASE_URLDesc=The+base+URL+of+the+domain+such+as+localhost&PORTDesc=The+port+such+as+8080&API_KEYDesc=Key+needed+to+access+this+API+%28NOT+a+Hypixel+API+key%29&POSTGRES_URLDesc=Full+URL+of+a+PostgreSQL+database&WEBHOOK_URLDesc=Discord+webhook+URL+for+logging&FEATURESDesc=The+features+%28QUERY%2C+PETS%2C+LOWESTBIN%29+you+want+enabled+separated+with+a+%27%2B%27&PORTDefault=8080&POSTGRES_URLDefault=%24%7B%7BDATABASE_URL%7D%7D&FEATURESDefault=QUERY%2BPETS%2BLOWESTBIN&referralCode=WrEybV)

## Todo
- Better documentation & more examples
- Improve underbin