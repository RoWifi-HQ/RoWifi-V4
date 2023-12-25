use rowifi_framework::{context::BotContext, error::FrameworkError};
use rowifi_models::{
    discord::cache::{CachedGuild, CachedMember},
    guild::PartialRoGuild,
    id::RoleId,
    user::RoUser,
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
    DenyList(String),
    InvalidNickname(String),
    Generic(FrameworkError),
}

impl UpdateUser<'_> {
    pub async fn execute(self) -> Result<UpdateUserSuccess, UpdateUserError> {
        let mut added_roles = Vec::<RoleId>::new();
        let mut removed_roles = Vec::<RoleId>::new();

        for unverified_role in &self.guild.unverified_roles {
            if self.server.roles.contains(unverified_role)
                && self.member.roles.contains(unverified_role)
            {
                removed_roles.push(*unverified_role);
            }
        }

        for verified_role in &self.guild.verified_roles {
            if self.server.roles.contains(verified_role)
                && self.member.roles.contains(verified_role)
            {
                added_roles.push(*verified_role);
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

        todo!()
    }
}

impl From<RobloxError> for UpdateUserError {
    fn from(err: RobloxError) -> Self {
        UpdateUserError::Generic(err.into())
    }
}
