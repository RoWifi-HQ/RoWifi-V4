use itertools::Itertools;
use rowifi_models::{
    bind::{AssetType, Bind},
    deny_list::{DenyList, DenyListData},
    discord::cache::{CachedGuild, CachedMember},
    guild::{BypassRoleKind, PartialRoGuild},
    id::RoleId,
    roblox::inventory::InventoryItem,
    user::RoUser,
};
use rowifi_roblox::{error::RobloxError, filter::AssetFilterBuilder, RobloxClient};
use std::collections::{HashMap, HashSet};
use twilight_http::Client as DiscordClient;

use crate::{
    custombinds::{
        self,
        evaluate::{EvaluationContext, EvaluationError, EvaluationResult},
    },
    error::RoError,
};

pub struct UpdateUser<'u> {
    pub http: &'u DiscordClient,
    pub roblox: &'u RobloxClient,
    pub member: &'u CachedMember,
    pub user: &'u RoUser,
    pub server: &'u CachedGuild,
    pub guild: &'u PartialRoGuild,
    pub all_roles: &'u [RoleId],
}

type UpdateUserSuccess = (Vec<RoleId>, Vec<RoleId>, String);

pub enum UpdateUserError {
    DenyList(DenyList),
    InvalidNickname(String),
    Generic(RoError),
    CustombindParsing { id: u32, err: String },
    CustombindEvaluation { id: u32, err: EvaluationError },
}

impl UpdateUser<'_> {
    #[allow(clippy::too_many_lines)]
    pub async fn execute(self) -> Result<UpdateUserSuccess, UpdateUserError> {
        let mut roles_to_add = Vec::<RoleId>::new();
        let mut roles_to_remove = Vec::<RoleId>::new();

        for unverified_role in &self.guild.unverified_roles {
            if self.server.roles.contains(unverified_role)
                && self.member.roles.contains(unverified_role)
            {
                roles_to_remove.push(*unverified_role);
            }
        }

        for verified_role in &self.guild.verified_roles {
            if self.server.roles.contains(verified_role)
                && self.member.roles.contains(verified_role)
            {
                roles_to_add.push(*verified_role);
            }
        }

        let user_id = self
            .user
            .linked_accounts
            .get(&self.guild.guild_id)
            .unwrap_or(&self.user.default_account_id);
        let user_ranks = self
            .roblox
            .get_user_roles(*user_id)
            .await?
            .into_iter()
            .map(|r| (r.group.id, r.role.rank))
            .collect::<HashMap<_, _>>();

        let roblox_user = self.roblox.get_user(*user_id).await?;

        let mut asset_filter = AssetFilterBuilder::new();
        for assetbind in &self.guild.assetbinds {
            match assetbind.asset_type {
                AssetType::Asset => asset_filter = asset_filter.asset(assetbind.asset_id),
                AssetType::Badge => asset_filter = asset_filter.badge(assetbind.asset_id),
                AssetType::Gamepass => asset_filter = asset_filter.gamepass(assetbind.asset_id),
            }
        }
        let asset_filter = asset_filter.build();
        let inventory_items = self
            .roblox
            .get_inventory_items(*user_id, asset_filter)
            .await?
            .into_iter()
            .map(|i| match i {
                InventoryItem::Asset(a) => a.asset_id,
                InventoryItem::Badge(b) => b.badge_id,
                InventoryItem::Gamepass(g) => g.gamepass_id,
            })
            .collect::<HashSet<_>>();

        let active_deny_lists = self
            .guild
            .deny_lists
            .iter()
            .filter(|d| match d.data {
                DenyListData::User(u) => u == roblox_user.id,
                DenyListData::Group(g) => user_ranks.contains_key(&g),
            })
            .sorted_by_key(|d| d.action_type)
            .last();
        if let Some(deny_list) = active_deny_lists {
            return Err(UpdateUserError::DenyList(deny_list.clone()));
        }

        let mut nickname_bind: Option<Bind> = None;

        for rankbind in &self.guild.rankbinds {
            // Check if the user's rank in the group is the same as the rankbind
            // or check if the bind is for the Guest role and the user is not in
            // the group
            let to_add = match user_ranks.get(&rankbind.group_id) {
                Some(rank_id) => *rank_id == rankbind.group_rank_id,
                None => rankbind.group_rank_id == 0,
            };
            if to_add {
                if let Some(ref highest) = nickname_bind {
                    if highest.priority() < rankbind.priority {
                        nickname_bind = Some(Bind::Rank(rankbind.clone()));
                    }
                    roles_to_add.extend(rankbind.discord_roles.iter().copied());
                }
            }
        }

        for groupbind in &self.guild.groupbinds {
            if user_ranks.contains_key(&groupbind.group_id) {
                if let Some(ref highest) = nickname_bind {
                    if highest.priority() < groupbind.priority {
                        nickname_bind = Some(Bind::Group(groupbind.clone()));
                    }
                    roles_to_add.extend(groupbind.discord_roles.iter().copied());
                }
            }
        }

        // TODO: Have parsed custombinds stored somewhere
        for custombind in &self.guild.custombinds {
            let exp = custombinds::parser::parser(&custombind.code).map_err(|err| {
                UpdateUserError::CustombindParsing {
                    id: custombind.custom_bind_id,
                    err: err.to_string(),
                }
            })?;
            let res = custombinds::evaluate::evaluate(
                &exp,
                &EvaluationContext {
                    roles: &self.member.roles,
                    ranks: &user_ranks,
                    username: &roblox_user.name,
                },
            )
            .map_err(|err| UpdateUserError::CustombindEvaluation {
                id: custombind.custom_bind_id,
                err,
            })?;
            let success = match res {
                EvaluationResult::Bool(b) => b,
                EvaluationResult::Number(n) => n != 0,
            };
            if success {
                if let Some(ref highest) = nickname_bind {
                    if highest.priority() < custombind.priority {
                        nickname_bind = Some(Bind::Custom(custombind.clone()));
                    }
                    roles_to_add.extend(custombind.discord_roles.iter().copied());
                }
            }
        }

        for assetbind in &self.guild.assetbinds {
            if inventory_items.contains(&assetbind.asset_id.0.to_string()) {
                if let Some(ref highest) = nickname_bind {
                    if highest.priority() < assetbind.priority {
                        nickname_bind = Some(Bind::Asset(assetbind.clone()));
                    }
                    roles_to_add.extend(assetbind.discord_roles.iter().copied());
                }
            }
        }

        let mut added_roles = Vec::new();
        let mut removed_roles = Vec::new();
        for bind_role in self.all_roles {
            if self.server.roles.contains(bind_role) {
                if roles_to_add.contains(bind_role) {
                    if !self.member.roles.contains(bind_role) {
                        added_roles.push(*bind_role);
                    }
                } else if self.member.roles.contains(bind_role) {
                    removed_roles.push(*bind_role);
                }
            }
        }

        let mut update = self
            .http
            .update_guild_member(self.server.id.0, self.member.id.0);

        let has_role_bypass = self
            .guild
            .bypass_roles
            .iter()
            .any(|b| b.kind == BypassRoleKind::Roles && self.member.roles.contains(&b.role_id));
        let mut new_roles = self.member.roles.clone();
        new_roles.extend_from_slice(&added_roles);
        new_roles.retain(|r| !removed_roles.contains(r));
        let new_roles = new_roles
            .into_iter()
            .unique()
            .map(|r| r.0)
            .collect::<Vec<_>>();
        // Check if the user has a roles bypass or if no roles are being added or removed
        if !has_role_bypass && (!added_roles.is_empty() || !removed_roles.is_empty()) {
            update = update.roles(&new_roles);
        }

        let original_nickname = self
            .member
            .nickname
            .as_ref()
            .map_or_else(|| self.member.username.as_str(), String::as_str);
        let has_nickname_bypass =
            self.guild.bypass_roles.iter().any(|b| {
                b.kind == BypassRoleKind::Nickname && self.member.roles.contains(&b.role_id)
            });
        let new_nickname = if let Some(nickname_bind) = nickname_bind {
            match nickname_bind {
                Bind::Rank(r) => {
                    r.template
                        .nickname(&roblox_user, self.user.user_id, &self.member.username)
                }
                Bind::Group(g) => {
                    g.template
                        .nickname(&roblox_user, self.user.user_id, &self.member.username)
                }
                Bind::Asset(a) => {
                    a.template
                        .nickname(&roblox_user, self.user.user_id, &self.member.username)
                }
                Bind::Custom(c) => {
                    c.template
                        .nickname(&roblox_user, self.user.user_id, &self.member.username)
                }
            }
        } else {
            self.guild.default_template.as_ref().unwrap().nickname(
                &roblox_user,
                self.user.user_id,
                &self.member.username,
            )
        };

        if !has_nickname_bypass && (original_nickname != new_nickname) {
            if new_nickname.len() > 32 {
                return Err(UpdateUserError::InvalidNickname(new_nickname));
            }

            update = update.nick(Some(&new_nickname));
        }

        let _res = update.await?;

        Ok((added_roles, removed_roles, new_nickname))
    }
}

impl From<RobloxError> for UpdateUserError {
    fn from(err: RobloxError) -> Self {
        UpdateUserError::Generic(err.into())
    }
}

impl From<twilight_http::Error> for UpdateUserError {
    fn from(err: twilight_http::Error) -> Self {
        UpdateUserError::Generic(err.into())
    }
}
