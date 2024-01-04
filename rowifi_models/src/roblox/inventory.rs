use serde::{de::Error as DeError, Deserialize, Deserializer};

use crate::bind::AssetType;

#[derive(Debug)]
pub enum InventoryItem {
    Asset(AssetDetails),
    Badge(BadgeDetails),
    Gamepass(GamepassDetails),
}

#[derive(Clone, Debug, Deserialize)]
pub struct AssetDetails {
    #[serde(rename = "assetId")]
    pub asset_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct BadgeDetails {
    #[serde(rename = "badgeId")]
    pub badge_id: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct GamepassDetails {
    #[serde(rename = "gamePassId")]
    pub gamepass_id: String,
}

#[allow(clippy::struct_field_names)]
#[derive(Deserialize)]
struct InventoryItemIntermediary {
    #[serde(rename = "assetDetails", default)]
    pub asset_details: Option<AssetDetails>,
    #[serde(rename = "badgeDetails", default)]
    pub badge_details: Option<BadgeDetails>,
    #[serde(rename = "gamePassDetails", default)]
    pub gamepass_details: Option<GamepassDetails>,
}

impl InventoryItem {
    #[must_use]
    pub const fn kind(&self) -> AssetType {
        match self {
            InventoryItem::Asset(_) => AssetType::Asset,
            InventoryItem::Badge(_) => AssetType::Badge,
            InventoryItem::Gamepass(_) => AssetType::Gamepass,
        }
    }
}

impl<'de> Deserialize<'de> for InventoryItem {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let intermediary = InventoryItemIntermediary::deserialize(deserializer)?;
        let item = if let Some(asset) = intermediary.asset_details {
            InventoryItem::Asset(asset)
        } else if let Some(badge) = intermediary.badge_details {
            InventoryItem::Badge(badge)
        } else if let Some(gamepass) = intermediary.gamepass_details {
            InventoryItem::Gamepass(gamepass)
        } else {
            return Err(DeError::unknown_variant(
                "InventoryItem",
                &["Asset", "Badge", "Gamepasss"],
            ));
        };
        Ok(item)
    }
}
