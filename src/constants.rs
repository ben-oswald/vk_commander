//! Application-wide constants for vk_commander.
//!
//! This module contains all magic numbers and configuration values
//! that are used throughout the application.

// =============================================================================
// Valkey/Redis Server Configuration
// =============================================================================

/// Minimum supported Valkey version (major, minor, patch)
pub const MIN_VALKEY_VERSION: (u8, u8, u8) = (5, 0, 0);

/// Minimum recommended Valkey version for optimal compatibility
pub const MIN_RECOMMENDED_VALKEY_VERSION: (u8, u8, u8) = (8, 0, 0);

/// Supported server types
pub const SUPPORTED_SERVERS: &[&str] = &["valkey"];

/// Partially supported server types (may have limited functionality)
pub const PARTIALLY_SUPPORTED_SERVERS: &[&str] = &["redis"];

/// Supported RESP protocols
pub const SUPPORTED_PROTOCOLS: &[&str] = &["RESP3"];

// =============================================================================
// Network & Connection
// =============================================================================

/// Default connection timeout in seconds
pub const CONNECTION_TIMEOUT_SECS: u64 = 5;

/// Default read/write timeout in seconds
pub const READ_WRITE_TIMEOUT_SECS: u64 = 10;

/// Maximum consecutive "would block" errors before timing out
pub const MAX_WOULD_BLOCK_ERRORS: usize = 3;

/// Buffer size for reading from network streams
pub const NETWORK_BUFFER_SIZE: usize = 8192;

// =============================================================================
// UI Layout
// =============================================================================

/// Minimum window dimensions (width, height)
pub const MIN_WINDOW_SIZE: (f32, f32) = (800.0, 600.0);

/// Default window dimensions (width, height)
pub const DEFAULT_WINDOW_SIZE: (f32, f32) = (1366.0, 768.0);

/// Banner display constants
pub const BANNER_FADEOUT_MS: u64 = 300;
pub const BANNER_MARGIN: f32 = 12.0;
pub const BANNER_WIDTH: f32 = 360.0;

/// Settings reload interval (in frames)
pub const SETTINGS_RELOAD_INTERVAL_FRAMES: u16 = 1024;

// =============================================================================
// Browser Window
// =============================================================================

/// Number of keys to fetch per SCAN operation
pub const SCAN_COUNT: usize = 500;

/// Debounce time for key metadata requests (milliseconds)
pub const KEY_METADATA_DEBOUNCE_MS: u64 = 300;

/// Maximum items to fetch when listing key contents (e.g., LRANGE, SMEMBERS)
pub const MAX_LIST_ITEMS: usize = 500;

// =============================================================================
// Workbench
// =============================================================================

/// Maximum characters to display in command response
pub const MAX_RESPONSE_CHARS: usize = 1024;

/// Maximum characters to display in command request preview
pub const MAX_REQUEST_CHARS: usize = 32;

/// Maximum entries in command history
pub const MAX_COMMAND_HISTORY: usize = 100;

// =============================================================================
// Banner Notifications
// =============================================================================

/// Default banner duration for error messages (milliseconds)
pub const BANNER_ERROR_DURATION_MS: u64 = 10_000;

/// Default banner duration for success messages (milliseconds)
pub const BANNER_SUCCESS_DURATION_MS: u64 = 3_000;

// =============================================================================
// RESP Protocol Error Prefixes
// =============================================================================

/// RESP error response prefixes that indicate server errors
pub const RESP_ERROR_PREFIXES: &[&str] = &[
    "BUSY ",
    "CLUSTERDOWN ",
    "ERR ",
    "EXECABORT ",
    "LOADING ",
    "MASTERDOWN ",
    "MISCONF ",
    "NOAUTH ",
    "NOPERM ",
    "NOREPLICAS ",
    "NOSCRIPT ",
    "OOM ",
    "READONLY ",
    "TRYAGAIN ",
    "WRONGPASS ",
    "WRONGTYPE ",
];