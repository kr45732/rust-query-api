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

A versatile API facade for the Hypixel Auction API written in Rust. The entire auction house is fetched every minute with NBT parsing and inserted into a PostgreSQL database in **less than a second** and with low memory usage (varies depending on enabled features, network speed, hardware, and latency of the Hypixel API)! You can query by auction UUID, auctioneer, end time, item name, item tier, item id, price, enchants, bin and bids. You can sort by the item's bin / starting price. You can track the average price of each unique pet-level-rarity combination. You can track the lowest prices of all bins. It also can track new bins that are at least one million lower than previous bins. It can track the average auction and average bin prices and sales for up to seven days with custom 'averaging methods'.

## Set Up
### Prerequisites
- [Rust](https://www.rust-lang.org/tools/install)
- [Cargo](https://doc.rust-lang.org/cargo/getting-started/installation.html)
- [PostgreSQL database](https://www.postgresql.org/)
- [Discord](https://discord.com/)

### Steps
- Clone the repository
- Rename the `.example_env` file to `.env` and fill out required fields **OR** set required fields using environment variables
- Run `cargo run --release` (may take some time to build)
- Use the API!

### Configuration Fields or Environment Variables
- `BASE_URL`: Base address to bind to (e.g. 0.0.0.0)
- `PORT`: Port to bind to (e.g. 8000)
  - Online hosts will automatically set this
- `API_KEY`: Optional key needed to access this API (NOT a Hypixel API key)
- `ADMIN_API_KEY`: Optional admin key required to use raw SQL parameters (defaults to the API_KEY)
- `POSTGRES_URL`: Full URL of a PostgreSQL database (should look like `postgres://[user]:[password]@[host]:[port]/[dbname]`)
- `WEBHOOK_URL`: Optional Discord webhook URL for logging
- `FEATURES`: Features (QUERY, PETS, LOWESTBIN, UNDERBIN, AVERAGE_AUCTION, AVERAGE_BIN) you want enabled separated with a '+' 
- `DEBUG`: If the API should log to files and stdout (defaults to false)

## Usage
### Endpoints
- `/query`
- `/pets`
- `/lowestbin`
- `/underbin`
- `/average_auction`
- `/average_bin`
- `/average`
- `/query_items`

### Documentation & Examples
- See documentation and examples [here](https://github.com/kr45732/rust-query-api/blob/main/docs/docs.md)

## Free Hosting
### Deploy On Railway
[![Deploy on Railway](https://railway.app/button.svg)](https://railway.app/new/template?template=https://github.com/kr45732/rust-query-api&plugins=postgresql&envs=BASE_URL,API_KEY,ADMIN_API_KEY,POSTGRES_URL,WEBHOOK_URL,FEATURES&optionalEnvs=WEBHOOK_URL,ADMIN_API_KEY&BASE_URLDesc=The+base+URL+of+the+domain.+Do+not+modify+this&API_KEYDesc=Key+needed+to+access+this+API+(NOT+a+Hypixel+API+key)&ADMIN_API_KEYDesc=Admin+key+required+to+use+raw+SQL+parameters.+Will+default+to+the+API_KEY+if+not+provided&POSTGRES_URLDesc=Full+URL+of+a+PostgreSQL+database.+No+need+to+modify+this+unless+you+are+using+your+own+database+since+Railway+already+provides+this+for+you.&WEBHOOK_URLDesc=Discord+webhook+URL+for+logging&FEATURESDesc=The+features+(QUERY,+PETS,+LOWESTBIN,+UNDERBIN,+AVERAGE_AUCTION,+AVERAGE_BIN)+you+want+enabled+separated+with+commas&BASE_URLDefault=0.0.0.0&POSTGRES_URLDefault=$%7B%7BDATABASE_URL%7D%7D&FEATURESDefault=QUERY,LOWESTBIN,AVERAGE_AUCTION,AVERAGE_BIN&referralCode=WrEybV)

### Deploy On Gigalixir
Steps to deploy on [Gigalixir](https://gigalixir.com/):
1. Clone repository
2. Install gigalixir CLI: `pip3 install gigalixir`
3. Sign up: `gigalixir signup`
4. Create app: `gigalixir create -n NAME`
5. Set environment variables: `gigalixir config:set key=value`
6. Deploy app: `git push gigalixir`
7. Acess at [https://NAME.gigalixirapp.com/](https://NAME.gigalixirapp.com/)

### Free PostgreSQL Datbase
The free tier of [Supabase](https://supabase.com/) is a great option with with plenty of storage and good performance.

## Todo
- Improve underbin
- Improve speed of database transactions