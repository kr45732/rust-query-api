# Docs
## Query
- `key` - key to access the API
- `query` - raw SQL to be executed. Requires the admin key
- `item_name` - filter auctions whose names contain this string
- `tier` - filter auctions by tier
- `item_id` - filter auctions by id
- `enchants` - an enchant the auction should have (only for enchanted books right now)
- `end` - filter auctions whose end time is after this (epoch timestamp in milliseconds)
- `bin` - filter if the auction should be a bin (true) or regular auction (false) or both (do not provide parameter)
- `bids` - filter auctions by the UUID of their bidders
- `sort` - sort by 'ASC' or 'DESC' bin price / starting price
- `limit` - max number of auctions returned (defaults to 1)

## Pets
- `key` - key to access the API
- `query` - list of pet names seperated with a comma. Each pet name is formated as: '[LVL_#]_NAME_TIER'. For tier boosted pets, append '_TB'. Requires the admin key

## Lowest bin
- `key` - key to access the API

## Under bin
- `key` - key to access the API

# Examples
### [example_1.json](https://github.com/kr45732/rust-query-api/blob/main/examples/example_1.json)
- Request: /query?key=KEY&bin=true&item_id=POWER_WITHER_CHESTPLATE&tier=MYTHIC&item_name=%✪✪✪✪✪%&sort=ASC&limit=50
- Meaning: find the cheapest 50 bins where the item id is POWER_WITHER_CHESTPLATE (Necron's chestplate), the tier is mythic, and has 5 stars. Sort by ascending bin price

### [example_2.json](https://github.com/kr45732/rust-query-api/blob/main/examples/example_2.json)
- Request: /pets?key=KEY&query='[LVL_100]_WITHER_SKELETON_LEGENDARY','[LVL_80]_BAL_EPIC','[LVL_96]_ENDER_DRAGON_EPIC_TB'
- Meaning: get the lowest/last pet prices for a level 100 legendary wither skeleton, a level 80 epic bal, and a level 96 epic ender dragon (tierboosted from epic to legendary)

### [example_3.json](https://github.com/kr45732/rust-query-api/blob/main/examples/example_3.json)
- Request /lowestbin?key=KEY
- Meaning: get all lowest bins

### [example_4.json](https://github.com/kr45732/rust-query-api/blob/main/examples/example_4.json)
- Request /underbin?key=KEY
- Meaning: get all new bins that were cheaper than the lowest bin of the previous API update. Experimental and still being improved.