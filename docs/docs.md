# Documentation
## Query
- `key` - key to access the API
- `query` - raw SQL to be executed. Requires the admin key
- `item_name` - filter auctions whose names contain this string
- `tier` - filter auctions by tier
- `item_id` - filter auctions by id
- `internal_id` - filter auctions by internal id
- `enchants` - an enchant the auction should have
- `end` - filter auctions whose end time is after this (epoch timestamp in milliseconds)
- `bin` - filter if the auction should be a bin (true) or regular auction (false) or both (do not provide parameter)
- `bids` - filter auctions by the UUID of their bidders
- `sort_by` - sort by 'starting_bid' or 'highest_bid'
- `sort_order` - sort 'ASC' or 'DESC'
- `limit` - max number of auctions returned (defaults to 1)

## Pets
- `key` - key to access the API
- `query` - comma seperated list of pet names. Each pet name is formated as: [LVL_#]_NAME_TIER. For tier boosted pets, append _TB

## Lowest Bin
- `key` - key to access the API

## Under Bin
- `key` - key to access the API

## Average Auctions
- `key` - key to access the API
- `time` - unix timestamp, in milliseconds, for how far back the average auction prices should be calculated. The most is 5 days back
- `step` - how the auction sales should be averaged. For example, 1 would average it by minute, 60 would average it by hour, 1440 would average it by day, and so on

## Average Bins
- `key` - key to access the API
- `time` - unix timestamp, in milliseconds, for how far back the average bin prices should be calculated. The most is 5 days back
- `step` - how the bin sales should be averaged. For example, 1 would average it by minute, 60 would average it by hour, 1440 would average it by day, and so on

## Average Auctions & Bins
- `key` - key to access the API
- `time` - unix timestamp, in milliseconds, for how far back the average auction & bin prices should be calculated. The most is 5 days back
- `step` - how the auction & bin sales should be averaged. For example, 1 would average it by minute, 60 would average it by hour, 1440 would average it by day, and so on

## Query Items
- `key` - key to access the API

# Examples
### [example_1.json](https://github.com/kr45732/rust-query-api/blob/main/docs/example_1.json)
- Request: /query?key=KEY&bin=true&item_id=POWER_WITHER_CHESTPLATE&tier=MYTHIC&item_name=%✪✪✪✪✪%&sort_by=starting_bid&sort_order=ASC&limit=50
- Meaning: find the cheapest 50 bins where the item id is POWER_WITHER_CHESTPLATE (Necron's chestplate), the tier is mythic, and has 5 stars. Sort by ascending bin price

### [example_2.json](https://github.com/kr45732/rust-query-api/blob/main/docs/example_2.json)
- Request: /pets?key=KEY&query=[LVL_100]_WITHER_SKELETON_LEGENDARY,[LVL_80]_BAL_EPIC,[LVL_96]_ENDER_DRAGON_EPIC_TB
- Meaning: get the average pet prices for a level 100 legendary wither skeleton, a level 80 epic bal, and a level 96 epic ender dragon (tier boosted from epic to legendary)

### [example_3.json](https://github.com/kr45732/rust-query-api/blob/main/docs/example_3.json)
- Request /lowestbin?key=KEY
- Meaning: get all lowest bins

### [example_4.json](https://github.com/kr45732/rust-query-api/blob/main/docs/example_4.json)
- Request /underbin?key=KEY
- Meaning: get all new bins that make at least one million in profit compared to the lowest bin of the previous API update. Experimental and still being improved

### [example_5.json](https://github.com/kr45732/rust-query-api/blob/main/docs/example_5.json)
- Request /average_auction?key=KEY&time=1647830293999&step=60
- Meaning: get average auction prices from the unix timestamp 1647830293999 to the present. Average sales by hour

### [example_6.json](https://github.com/kr45732/rust-query-api/blob/main/docs/example_6.json)
- Request /average_bin?key=KEY&time=1647830293999&step=60
- Meaning: get average auction bin from the unix timestamp 1647830293999 to the present. Average sales by hour

### [example_7.json](https://github.com/kr45732/rust-query-api/blob/main/docs/example_7.json)
- Request /average?key=KEY&time=1647830293999&step=60
- Meaning: get the combined average auctions and average bins from the unix timestamp 1647830293999 to the present. Average sales by hour

### [example_8.json](https://github.com/kr45732/rust-query-api/blob/main/docs/example_8.json)
- Request /query_items?key=KEY
- Meaning: get a list of all current unique auction names