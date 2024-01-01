use itertools::Itertools;
use rowifi_framework::{context::BotContext, error::FrameworkError};
use rowifi_models::{
    discord::cache::{CachedGuild, CachedMember},
    guild::PartialRoGuild,
    id::RoleId,
    user::RoUser, deny_list::{DenyList, DenyListData}, bind::Bind,
};
use rowifi_roblox::error::RobloxError;
use std::collections::HashMap;

pub struct UpdateUser<'u> {
    pub ctx: &'u BotContext,
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
    Generic(FrameworkError),
}

impl UpdateUser<'_> {
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
            .ctx
            .roblox
            .get_user_roles(*user_id)
            .await?
            .into_iter()
            .map(|r| (r.group.id, r.role.rank))
            .collect::<HashMap<_, _>>();

        let roblox_user = self.ctx.roblox.get_user(*user_id).await?;

        let active_deny_lists = self.guild.deny_lists.0.iter()
            .filter(|d| {
                match d.data {
                    DenyListData::User(u) => u == roblox_user.id,
                    DenyListData::Group(g) => user_ranks.contains_key(&g)
                }
            })
            .sorted_by_key(|d| d.action_type)
            .last();
        if let Some(deny_list) = active_deny_lists {
            return Err(UpdateUserError::DenyList(deny_list.clone()));
        }

        let mut nickname_bind: Option<Bind> = None;

        for rankbind in &self.guild.rankbinds.0 {
            // Check if the user's rank in the group is the same as the rankbind
            // or check if the bind is for the Guest role and the user is not in
            // the group
            let to_add = match user_ranks.get(&rankbind.group_id) {
                Some(rank_id) => *rank_id == rankbind.group_rank_id   ,
                None => rankbind.group_rank_id == 0
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

        for groupbind in &self.guild.groupbinds.0 {
            let to_add = user_ranks.contains_key(&groupbind.group_id);
            if to_add {
                if let Some(ref highest) = nickname_bind {
                    if highest.priority() < groupbind.priority {
                        nickname_bind = Some(Bind::Group(groupbind.clone()));
                    }
                    roles_to_add.extend(groupbind.discord_roles.iter().copied());
                }
            }
        }

        for assetbind in &self.guild.assetbinds.0 {
            
        }

        todo!()
    }
}

impl From<RobloxError> for UpdateUserError {
    fn from(err: RobloxError) -> Self {
        UpdateUserError::Generic(err.into())
    }
}
