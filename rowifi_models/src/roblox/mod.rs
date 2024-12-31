pub mod group;
pub mod id;
pub mod inventory;
pub mod user;
pub mod universe;

use serde::Deserialize;

/// Represents a long-running operation
///
/// See [`Operation`](https://create.roblox.com/docs/reference/cloud/assets/v1#Operation) for details.
#[derive(Deserialize)]
pub struct Operation<T> {
    pub response: T,
}
