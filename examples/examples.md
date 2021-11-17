# Examples
### [example_1.json](https://github.com/kr45732/rust-query-api/blob/main/examples/example_1.json)
- Request: /query?query=item_id=item_id='POWER_WITHER_CHESTPLATE' AND tier='MYTHIC' AND item_name LIKE '%✪✪✪✪✪%'&sort=starting_bid
- Meaning: find all auctions where the item id is POWER_WITHER_CHESTPLATE (Necron's chestplate), the tier is mythic, and has 5 stars. Sort by ascending bin price

### [example_2.json](https://github.com/kr45732/rust-query-api/blob/main/examples/example_2.json)
- Request: /pets?query='[LVL_100]_WITHER_SKELETON_LEGENDARY','[LVL_80]_BAL_EPIC','[LVL_25]_ROCK_COMMON'
- Meaning: get the lowest/last pet prices for a level 100 legendary wither skeleton, a level 80 epic bal, and a level 25 common rock

### [example_3.json](https://github.com/kr45732/rust-query-api/blob/main/examples/example_3.json)
- Request /lowestbin
- Meaning: get all lowest bins