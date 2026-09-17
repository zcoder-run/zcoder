// region:    --- Modules

mod socket_path;
// The wire adapters are consumed by the client and server transports added in the next steps.
#[allow(dead_code)]
mod wire;
// The client connection is consumed by RouterClient::uds added in the next step.
#[cfg(feature = "client")]
#[allow(dead_code)]
mod client_conn;

#[allow(unused_imports)]
pub(crate) use wire::MAX_FRAME_LEN;
pub(crate) use wire::{WireReader, WireWriter};
#[cfg(feature = "client")]
pub(crate) use client_conn::{ClientConn, ClientConnSink};
pub use socket_path::{is_live, socket_path, unlink_if_exists};

// endregion: --- Modules
