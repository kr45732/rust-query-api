# Docs
## Query
- `key` - key to access the API
- `query` - raw SQL to be executed (will soon be depreciated or locked behind an admin key to prevent SQL injection)
- `item_name` - filter auctions whose names contain this string
- `tier` - filter auctions by tier
- `item_id` - filter auctions by id
- `enchants` - an enchant the auction should have (only works for enchanted books right now)
- `end` - filter auctions whose end time is after this (epoch timestamp in milliseconds)
- `sort` - name of column to sort by (ascending)
- `limit` - number of items returned

## Pets
- `key` - key to access the API
- `query` - list of pet names seperated with a comma. Each pet name is formated as: '[LVL_#]_NAME_TIER'. For tier boosted pets, append '_TB'

## Lowest bin
- `key` - key to access the API

# Examples
### [example_1.json](https://github.com/kr45732/rust-query-api/blob/main/examples/example_1.json)
- Request: /query?key=KEY&item_id=POWER_WITHER_CHESTPLATE&tier=MYTHIC&item_name=%✪✪✪✪✪%&sort=starting_bid
- Meaning: find all auctions where the item id is POWER_WITHER_CHESTPLATE (Necron's chestplate), the tier is mythic, and has 5 stars. Sort by ascending bin price

### [example_2.json](https://github.com/kr45732/rust-query-api/blob/main/examples/example_2.json)
- Request: /pets?key=KEY&query='[LVL_100]_WITHER_SKELETON_LEGENDARY','[LVL_80]_BAL_EPIC','[LVL_25]_ROCK_COMMON'
- Meaning: get the lowest/last pet prices for a level 100 legendary wither skeleton, a level 80 epic bal, and a level 25 common rock

### [example_3.json](https://github.com/kr45732/rust-query-api/blob/main/examples/example_3.json)
- Request /lowestbin?key=KEY
- Meaning: get all lowest bins