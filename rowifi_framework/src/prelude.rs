pub use crate::arguments::{Argument, Arguments};
pub use crate::context::{BotContext, CommandContext, DeferredResponse};
pub use crate::spawn_command;
pub use crate::Command;

pub use axum::{response::IntoResponse, Extension, Json};
pub use rowifi_core::error::RoError;
pub use rowifi_derive::Arguments;
pub use time::OffsetDateTime;
pub use twilight_util::builder::{
    embed::{EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder, ImageSource},
    InteractionResponseDataBuilder,
};

pub type CommandResult = Result<(), RoError>;

pub const RED: u32 = 0x00E7_4C3C;
pub const DARK_GREEN: u32 = 0x001F_8B4C;
pub const BLUE: u32 = 0x0034_98DB;
