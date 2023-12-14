pub use crate::context::{CommandContext, DeferredResponse};
pub use crate::error::FrameworkError;
pub use crate::arguments::{Arguments, Argument};

pub use time::OffsetDateTime;
pub use twilight_util::builder::embed::{EmbedBuilder, EmbedFieldBuilder, EmbedFooterBuilder};
pub use rowifi_derive::Arguments;

pub type CommandResult = Result<(), FrameworkError>;

pub const RED: u32 = 0x00E7_4C3C;