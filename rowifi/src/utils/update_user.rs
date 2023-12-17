use rowifi_framework::{context::BotContext, error::FrameworkError};
use rowifi_models::{
    bind::Bind,
    discord::cache::{CachedGuild, CachedMember},
    id::RoleId,
    user::RoUser, guild::PartialRoGuild,
};
use std::collections::HashSet;

pub struct UpdateUser<'u> {
    pub ctx: &'u BotContext,
    pub member: &'u CachedMember,
    pub user: &'u RoUser,
    pub server: &'u CachedGuild,
    pub guild: &'u PartialRoGuild,
    pub binds: &'u [Bind],
    pub guild_roles: &'u HashSet<RoleId>,
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
            if self.guild_roles.get(unverified_role).is_some() && self.member.roles.contains(unverified_role) {
                removed_roles.push(*unverified_role);
            }
        }

        todo!()
    }
}
