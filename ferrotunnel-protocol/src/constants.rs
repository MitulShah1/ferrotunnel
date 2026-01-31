//! Protocol constants

/// Current protocol version
pub const PROTOCOL_VERSION: u8 = 1;

/// Minimum supported protocol version (for backward compatibility)
pub const MIN_PROTOCOL_VERSION: u8 = 1;

/// Maximum supported protocol version (for forward compatibility)
pub const MAX_PROTOCOL_VERSION: u8 = 1;

/// Maximum frame size (16MB)
pub const MAX_FRAME_SIZE: u32 = 16 * 1024 * 1024;

/// Heartbeat interval in seconds
pub const HEARTBEAT_INTERVAL_SECS: u64 = 30;

/// Connection timeout in seconds
pub const CONNECTION_TIMEOUT_SECS: u64 = 90;
