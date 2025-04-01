#![deny(clippy::all, clippy::pedantic)]
#![allow(
    clippy::module_name_repetitions,
    clippy::implicit_hasher,
    clippy::similar_names,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

pub mod assetbinds;
pub mod backups;
pub mod custombinds;
pub mod denylists;
pub mod error;
pub mod events;
pub mod groupbinds;
pub mod rankbinds;
pub mod user;
pub mod custom;
