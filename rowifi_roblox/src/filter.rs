use itertools::Itertools;
use rowifi_models::roblox::id::AssetId;

#[derive(Default)]
pub struct AssetFilterBuilder {
    asset_ids: Vec<AssetId>,
    badge_ids: Vec<AssetId>,
    gamepass_ids: Vec<AssetId>,
}

impl AssetFilterBuilder {
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    #[must_use]
    pub fn asset(mut self, asset_id: AssetId) -> Self {
        self.asset_ids.push(asset_id);
        self
    }

    #[must_use]
    pub fn badge(mut self, badge_id: AssetId) -> Self {
        self.badge_ids.push(badge_id);
        self
    }

    #[must_use]
    pub fn gamepass(mut self, gamepass_id: AssetId) -> Self {
        self.gamepass_ids.push(gamepass_id);
        self
    }

    #[must_use]
    pub fn build(self) -> String {
        let mut filters = Vec::new();

        if !self.asset_ids.is_empty() {
            let filter = String::from("assetIds=");
            let asset_ids = self.asset_ids.into_iter().join(",");
            filters.push(filter + &asset_ids);
        }

        if !self.badge_ids.is_empty() {
            let filter = String::from("badgeIds=");
            let badge_ids = self.badge_ids.into_iter().join(",");
            filters.push(filter + &badge_ids);
        }

        if !self.gamepass_ids.is_empty() {
            let filter = String::from("gamePassIds=");
            let gamepass_ids = self.gamepass_ids.into_iter().join(",");
            filters.push(filter + &gamepass_ids);
        }

        let filter = String::from("filter=") + filters.join(";").as_str();
        filter
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.asset_ids.is_empty() && self.badge_ids.is_empty() && self.gamepass_ids.is_empty()
    }
}
