pub use crate::arguments::{Argument, Arguments};
pub use crate::context::{CommandContext, DeferredResponse};
pub use crate::error::FrameworkError;

pub use rowifi_derive::Arguments;
pub use time::OffsetDateTime;
pub use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder};

pub type CommandResult = Result<(), FrameworkError>;

pub const RED: u32 = 0x00E7_4C3C;
pub const DARK_GREEN: u32 = 0x001F_8B4C;
