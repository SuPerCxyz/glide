pub mod client_connection;
pub mod clipboard;
/// Core types for the Glide clipboard sync protocol.
///
/// This crate owns all shared data types: device identity, clipboard items,
/// MIME representations, payload references, transfer sessions, sync events,
/// and input events.
pub mod device;
pub mod discovery;
pub mod display;
pub mod display_layout;
pub mod error;
pub mod input_event;
pub mod input_config;
pub mod mime_rep;
pub mod payload;
pub mod policy;
pub mod route;
pub mod sync_event;
pub mod transfer;

pub use client_connection::*;
pub use clipboard::*;
pub use device::*;
pub use display_layout::*;
pub use error::*;
pub use input_config::*;
pub use input_event::*;
pub use mime_rep::*;
pub use payload::*;
pub use policy::*;
pub use sync_event::*;
pub use transfer::*;
