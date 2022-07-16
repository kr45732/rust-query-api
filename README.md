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

A versatile API facade for the Hypixel Auction API written in Rust. The entire auction house is fetched with NBT parsing and inserted into a PostgreSQL database in about 3-7 seconds every minute with low memory usage (can vary depending on enabled features, network speed, and latency of the Hypixel API)! You can query by auction UUID, auctioneer, end time, item name, item tier, item id, price, enchants, bin and bids. You can sort by the item's bin / starting price. You can track the average price of each unique pet-level-rarity combination. You can track the lowest prices of all bins. It also can track new bins that are at least one million lower than previous bins. Lastly, it can track the average auction prices and sales up to five days with custom 'averaging methods'.

## Set Up
### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
- [PostgreSQL database](https://www.postgresql.org/)
- [Discord](https://discord.com/)

### Steps
- Clone the repository
- Rename the `.example_env` file to `.env` and fill out all fields **OR** set all fields using environment variables
- Run `cargo run --release` (may take some time to build)
- Use it!

### Configuration Fields or Environment Variables
- `BASE_URL`: The base URL of the domain such as 127.0.0.1
- `PORT`: The port such as 8080
- `API_KEY`: Key needed to access this API (NOT a Hypixel API key)
- `ADMIN_API_KEY`: Admin key required to use raw SQL parameters. Will default to the API_KEY if not provided
- `POSTGRES_URL`: Full URL of a PostgreSQL database
- `WEBHOOK_URL`: Discord webhook URL for logging
- `FEATURES`: The features (QUERY, PETS, LOWESTBIN, UNDERBIN, AVERAGE_AUCTION, AVERAGE_BIN) you want to be enabled separated with a '+' 

## Usage
### Endpoints
- `/query`
- `/pets`
- `/lowestbin`
- `/underbin`
- `/average_auction`
- `/average_bin`

### Docs & Examples
- See docs and examples [here](https://github.com/kr45732/rust-query-api/blob/main/examples/examples.md)

## Free Hosting
### Deploy To Heroku (Recommended)
[![Deploy](https://www.herokucdn.com/deploy/button.svg)](https://heroku.com/deploy)

### Free PostgreSQL Datbase
For a free PostgreSQL database host, [Supabase](https://supabase.com/) is a really good choice and offers two free databases with plenty of space and performance.

### Deploy To Railway
[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/new/template?template=https://github.com/kr45732/rust-query-api&plugins=postgresql&envs=BASE_URL,API_KEY,ADMIN_API_KEY,POSTGRES_URL,WEBHOOK_URL,FEATURES&optionalEnvs=WEBHOOK_URL,ADMIN_API_KEY&BASE_URLDesc=The+base+URL+of+the+domain.+Do+not+modify+this&API_KEYDesc=Key+needed+to+access+this+API+(NOT+a+Hypixel+API+key)&ADMIN_API_KEYDesc=Admin+key+required+to+use+raw+SQL+parameters.+Will+default+to+the+API_KEY+if+not+provided&POSTGRES_URLDesc=Full+URL+of+a+PostgreSQL+database.+No+need+to+modify+this+unless+you+are+using+your+own+database+since+Railway+already+provides+this+for+you.&WEBHOOK_URLDesc=Discord+webhook+URL+for+logging&FEATURESDesc=The+features+(QUERY,+PETS,+LOWESTBIN,+UNDERBIN,+AVERAGE_AUCTION,+AVERAGE_BIN)+you+want+enabled+separated+with+commas&BASE_URLDefault=0.0.0.0&POSTGRES_URLDefault=$%7B%7BDATABASE_URL%7D%7D&FEATURESDefault=QUERY,LOWESTBIN,AVERAGE_AUCTION,AVERAGE_BIN&referralCode=WrEybV)

## Todo
- Better documentation & more examples
- Improve underbin