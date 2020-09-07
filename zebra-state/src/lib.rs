//! State storage code for Zebra. 🦓
//!
//! ## Organizational Structure
//!
//! zebra-state tracks `Blocks` using two key-value trees
//!
//! * BlockHeaderHash -> Block
//! * BlockHeight -> Block
//!
//! Inserting a block into the service will create a mapping in each tree for that block.
#![doc(html_logo_url = "https://www.zfnd.org/images/zebra-icon.png")]
#![doc(html_root_url = "https://doc.zebra.zfnd.org/zebra_state")]
#![warn(missing_docs)]
#![allow(clippy::try_err)]
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use zebra_chain::block::{Block, BlockHeaderHash, BlockHeader};

pub mod on_disk;
pub mod on_disk_headersonly;
pub mod in_memory;
pub mod in_memory_headersonly;

/// Configuration for networking code.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Config {
    /// The root directory for the state storage
    pub path: PathBuf,
}

impl Config {
    pub(crate) fn sled_config(&self) -> sled::Config {
        sled::Config::default().path(&self.path)
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            path: PathBuf::from("./.zebra-state"),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// A state request, used to manipulate the zebra-state on disk or in memory
pub enum RequestBlock {
    // TODO(jlusby): deprecate in the future based on our validation story
    /// Add a block to the zebra-state
    AddBlock {
        /// The block to be added to the state
        block: Arc<Block>,
    },
    /// Get a block from the zebra-state
    GetBlock {
        /// The hash used to identify the block
        hash: BlockHeaderHash,
    },
    /// Get the block that is the tip of the current chain
    GetTip,
    /// Ask the state if the given hash is part of the current best chain
    GetDepth {
        /// The hash to check against the current chain
        hash: BlockHeaderHash,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// A state request, used to manipulate the zebra-state on disk or in memory
pub enum RequestBlockHeader {
    /// Add a block header to the zebra-state
    AddBlockHeader {
        /// The block header to be added to the state
        block_header: Arc<BlockHeader>,
    },
    /// Get a block header from the zebra-state
    GetBlockHeader {
        /// The hash used to identify the block header
        hash: BlockHeaderHash,
    },
    /// Get the block that is the tip of the current chain
    GetTip,
    /// Ask the state if the given hash is part of the current best chain
    GetDepth {
        /// The hash to check against the current chain
        hash: BlockHeaderHash,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
/// A state response
pub enum Response {
    /// The response to a `AddBlock` request indicating a block was successfully
    /// added to the state
    Added {
        /// The hash of the block that was added
        hash: BlockHeaderHash,
    },
    /// The response to a `GetBlock` request by hash
    Block {
        /// The block that was requested
        block: Arc<Block>,
    },
    /// The response to a `GetBlockHeader` request by hash
    BlockHeader {
        /// The block header that was requested
        block_header: Arc<BlockHeader>,
    },
    /// The response to a `GetTip` request
    Tip {
        /// The hash of the block at the tip of the current chain
        hash: BlockHeaderHash,
    },
    /// The response to a `Contains` request indicating that the given has is in
    /// the current best chain
    Depth(
        /// The number of blocks above the given block in the current best chain
        Option<u32>,
    ),
}
