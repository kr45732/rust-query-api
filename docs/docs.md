# Documentation
## Query
- `key` - key to access the API
- `query` - raw SQL to be executed. Requires the admin key
- `item_name` - filter by name
- `tier` - filter by tier
- `item_id` - filter by id
- `internal_id` - filter by internal id
- `enchants` - filter by comma separated list of enchants
- `end` - filter if end time is after this (epoch timestamp in milliseconds)
- `bin` - filter by bin (true) or regular auction (false) or any (do not provide parameter)
- `potato_books` - filter by potato books count (hot and fuming potato books are combined)
- `stars` - filter by number of stars
- `farming_for_dummies` - filter by farming for dummies count
- `transmission_tuner` - filter by transmission tuner count
- `mana_disintegrator` - filter by mana disintegrator count
- `reforge` - filter by reforge name
- `rune` - filter by rune
- `skin` - filter by item skin
- `power_scroll` - filter by power scroll
- `drill_upgrade_module` - filter by drill upgrade module
- `drill_fuel_tank` - filter by drill fuel tank
- `drill_engine` - filter by drill engine
- `dye` - filter by dye
- `accessory_enrichment` - filter by accessory enrichment
- `recombobulated` - filter by recombobulator applied
- `wood_singularity` - filter by wood singularity applied
- `art_of_war` = filter by art of war applied
- `art_of_peace` = filter by art of peace applied
- `etherwarp` - filter by etherwarp applied
- `necron_scrolls` - filter by comma separated list of necron scrolls
- `gemstones` - filter by comma separated list of gemstones. Each gemstone is formatted as SLOT_GEMSTONE (e.g. JADE_0_FINE_JADE_GEM)
- `bids` - filter auctions by the UUID of their bidders
- `sort_by` - sort by 'starting_bid' or 'highest_bid', or 'query'. Sorting by query will return a score indicating the number conditions an item matched
- `sort_order` - sort 'ASC' or 'DESC'
- `limit` - max number of auctions returned (defaults to 1). Limit of 0 will return return all auctions. Limits not between 0 and 500 require the admin key

## Pets
- `key` - key to access the API
- `query` - comma separated list of pet names. Each pet name is formatted as: [LVL_#]_NAME_TIER. For tier boosted pets, append _TB

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
### [Query Example #1](docs/query_example_1.json)
- Request: /query?key=KEY&bin=true&item_id=POWER_WITHER_CHESTPLATE&recombobulated=true&stars=5&sort_by=starting_bid&sort_order=ASC&limit=50
- Meaning: find the cheapest 50 bins where the item id is POWER_WITHER_CHESTPLATE, is recombobulated, and has 5 stars. Sort by ascending bin price

### [Query Example #2](docs/query_example_2.json)
- Request: /query?key=KEY&bin=true&item_id=POWER_WITHER_CHESTPLATE&recombobulated=true&enchants=GROWTH;6&gemstones=COMBAT_0_FINE_JASPER_GEM&stars=5&sort_by=query&limit=50
- Meaning: find the closest matching bins where the item id is POWER_WITHER_CHESTPLATE, is recombobulated, enchanted with growth 6, have a fine jasper in the combat gemstone slot, and has 5 stars. Sort by ascending bin price and limit to 50 results. Returns a score indicating number of conditions matched

### [Pets Example](docs/pets_example.json)
- Request: /pets?key=KEY&query=[LVL_100]_WITHER_SKELETON_LEGENDARY,[LVL_80]_BAL_EPIC,[LVL_96]_ENDER_DRAGON_EPIC_TB
- Meaning: get the average pet prices for a level 100 legendary wither skeleton, a level 80 epic bal, and a level 96 epic ender dragon (tier boosted from epic to legendary)

### [Lowestbin Example](docs/lowestbin_example.json)
- Request /lowestbin?key=KEY
- Meaning: get all lowest bins

### [Underbin Example](docs/underbin_example.json)
- Request /underbin?key=KEY
- Meaning: get all new bins that make at least one million in profit compared to the lowest bin of the previous API update. Experimental and still being improved

### [Average Auction Example](docs/average_auction_example.json)
- Request /average_auction?key=KEY&time=1647830293999&step=60
- Meaning: get average auction prices from the unix timestamp 1647830293999 to the present. Average sales by hour

### [Average Bin Example](docs/average_bin_example.json)
- Request /average_bin?key=KEY&time=1647830293999&step=60
- Meaning: get average auction bin from the unix timestamp 1647830293999 to the present. Average sales by hour

### [Average Example](docs/average_example.json)
- Request /average?key=KEY&time=1647830293999&step=60
- Meaning: get the combined average auctions and average bins from the unix timestamp 1647830293999 to the present. Average sales by hour

### [Query Items Example](docs/query_items_example.json)
- Request /query_items?key=KEY
- Meaning: get a list of all current unique auction names