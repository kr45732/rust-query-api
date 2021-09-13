use nbt::from_gzip_reader;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::io;
use std::result::Result as StdResult;

#[derive(Deserialize)]
pub struct PartialNbt {
    pub i: Vec<PartialNbtElement>,
}

#[derive(Deserialize)]
pub struct PartialNbtElement {
    #[serde(rename = "Count")]
    pub count: i64,
    pub tag: PartialTag,
}

#[derive(Deserialize)]
pub struct PartialTag {
    #[serde(rename = "ExtraAttributes")]
    pub extra_attributes: PartialExtraAttr,
    pub display: DisplayInfo,
}

#[derive(Serialize, Deserialize)]
pub struct Pet {
    #[serde(rename = "type")]
    pub pet_type: String,

    #[serde(rename = "tier")]
    pub tier: String,
}

#[derive(Deserialize)]
pub struct PartialExtraAttr {
    pub id: String,
    #[serde(rename = "petInfo")]
    pub pet: Option<String>,
    pub enchantments: Option<HashMap<String, i32>>,
    pub potion: Option<String>,
    pub potion_level: Option<i16>,
    pub anvil_uses: Option<i16>,
    pub enhanced: Option<bool>,
    pub runes: Option<HashMap<String, i32>>,
}

#[derive(Deserialize)]
pub struct DisplayInfo {
    #[serde(rename = "Name")]
    pub name: String,
    #[serde(rename = "Lore")]
    pub lore: Vec<String>,
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
pub struct Item {
    #[serde(rename = "item_name")]
    pub item_name: String,
    #[serde(rename = "item_lore")]
    pub item_lore: String,
    #[serde(rename = "uuid")]
    pub uuid: String,
    #[serde(rename = "auctioneer")]
    pub auctioneer: String,
    #[serde(rename = "end")]
    pub end: i64,
    #[serde(rename = "item_count", skip_serializing_if = "Option::is_none")]
    pub item_count: Option<i64>,
    #[serde(rename = "tier")]
    pub tier: String,
    #[serde(rename = "item_bytes")]
    pub item_bytes: ItemBytes,
    #[serde(rename = "starting_bid")]
    pub starting_bid: i64,
    #[serde(rename = "bin")]
    pub bin: Option<bool>,
}

impl Item {
    pub fn to_nbt(&self) -> Result<PartialNbt, Box<dyn std::error::Error>> {
        let bytes: StdResult<Vec<u8>, _> = self.item_bytes.clone().into();
        let nbt: PartialNbt = from_gzip_reader(io::Cursor::new(bytes?))?;
        Ok(nbt)
    }

    /// Returns the count of items in the stack.
    /// Attempts to count the items in the stack if no cached version is available.
    /// Returns None otherwise
    pub fn count(&mut self) -> Option<i64> {
        if let Some(ref count) = &self.item_count {
            return Some(*count);
        }

        if let Ok(nbt) = self.to_nbt() {
            if let Some(pnbt) = nbt.i.first() {
                self.item_count = Some(pnbt.count);

                return Some(pnbt.count);
            }
        }

        None
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(untagged)]
pub enum ItemBytes {
    T0(ItemBytesT0),
    Data(String),
}

impl Into<String> for ItemBytes {
    fn into(self) -> String {
        match self {
            Self::T0(ibt0) => {
                let ItemBytesT0::Data(x) = ibt0;
                x
            }
            Self::Data(x) => x,
        }
    }
}

impl Into<Result<Vec<u8>, Box<dyn std::error::Error>>> for ItemBytes {
    fn into(self) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let b64: String = self.into();
        Ok(base64::decode(&b64)?)
    }
}

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq, Clone)]
#[serde(tag = "type", content = "data")]
pub enum ItemBytesT0 {
    #[serde(rename = "0")]
    Data(String),
}
