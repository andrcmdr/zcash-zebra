//! Definitions of constants.

use crate::types::*;

/// The User-Agent string provided by the node.
pub const USER_AGENT: &'static str = "Zebra v2.0.0-alpha.0";

/// The Zcash network protocol version used on mainnet.
pub const CURRENT_VERSION: Version = Version(170_007);

/// The minimum version supported for peer connections.
pub const MIN_VERSION: Version = Version(170_007);

/// Magic numbers used to identify different Zcash networks.
pub mod magics {
    use super::*;
    /// The production mainnet.
    pub const MAINNET: Magic = Magic([0x24, 0xe9, 0x27, 0x64]);
    /// The testnet.
    pub const TESTNET: Magic = Magic([0xfa, 0x1a, 0xf9, 0xbf]);
}