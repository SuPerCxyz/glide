/// Core types for the Glide clipboard sync protocol.
///
/// This crate owns all shared data types: device identity, clipboard items,
/// MIME representations, payload references, transfer sessions, sync events,
/// and input events.

pub mod device;
pub mod clipboard;
pub mod mime_rep;
pub mod payload;
pub mod transfer;
pub mod sync_event;
pub mod input_event;
pub mod policy;
pub mod error;

pub use clipboard::*;
pub use device::*;
pub use error::*;
pub use input_event::*;
pub use mime_rep::*;
pub use payload::*;
pub use policy::*;
pub use sync_event::*;
pub use transfer::*;
