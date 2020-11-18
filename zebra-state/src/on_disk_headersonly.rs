//! The primary implementation of the `zebra_state::Service` built upon sled
use super::{RequestBlockHeader, Response, QueryType};
use crate::Config;
// use std::path::{Path, PathBuf};
use futures::prelude::*;
use std::sync::Arc;
use std::{
    error,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{buffer::Buffer, Service};
use zebra_chain::serialization::{ZcashDeserialize, ZcashSerialize};
use zebra_chain::{
    block::{BlockHeader, BlockHeaderHash},
    types::BlockHeight,
};

type Error = Box<dyn error::Error + Send + Sync + 'static>;

#[derive(Clone)]
struct SledState {
    storage: sled::Db,
}

impl SledState {
    pub(crate) fn new(config: &Config) -> Self {
        let config = config.sled_config("headers");
        Self {
            storage: config.open().unwrap(),
        }
    }

    pub(super) fn insert(
        &mut self,
        block_header: impl Into<Arc<BlockHeader>>,
        block_height: BlockHeight,
    ) -> Result<(BlockHeaderHash, BlockHeight), Error> {
        let block_header = block_header.into();
        let hash: BlockHeaderHash = block_header.as_ref().into();
//      let height = block.coinbase_height().unwrap(); // didn't applicable for block height handling
        let height = block_height;

        let by_hash = self.storage.open_tree(b"by_hash")?;
        let by_height = self.storage.open_tree(b"by_height")?;
        let hash_height = self.storage.open_tree(b"hash_height")?;

        let mut bytes = Vec::new();
        block_header.zcash_serialize(&mut bytes)?;

        // TODO(jlusby): make this transactional
        by_hash.insert(&hash.0, bytes.as_slice())?;
        by_height.insert(&height.0.to_be_bytes(), bytes.as_slice())?;
        hash_height.insert(&hash.0, &height.0.to_be_bytes())?;

        Ok((hash, height))
    }

    pub(super) fn get(&self, query: impl Into<QueryType>) -> Result<Option<Arc<BlockHeader>>, Error> {
        let query = query.into();
        let value = match query {
            QueryType::ByHash(hash) => {
                let by_hash = self.storage.open_tree(b"by_hash")?;
                let key = &hash.0;
                by_hash.get(key)?
            }
            QueryType::ByHeight(height) => {
                let by_height = self.storage.open_tree(b"by_height")?;
                let key = &height.0.to_be_bytes();
                by_height.get(key)?
            }
        };

        if let Some(bytes) = value {
            let bytes = bytes.as_ref();
            let block_header = ZcashDeserialize::zcash_deserialize(bytes)?;
            Ok(Some(block_header))
        } else {
            Ok(None)
        }
    }

    pub(super) fn get_tip(&self) -> Result<Option<(Arc<BlockHeader>, BlockHeaderHash, BlockHeight)>, Error> {
        let by_height = self.storage.open_tree(b"by_height")?;
        let last_entry = by_height.iter().values().next_back();
        let last_entry_height = by_height.iter().keys().next_back();

        match (&last_entry, &last_entry_height) {
            (Some(Ok(last_entry_bytes)), Some(Ok(last_entry_height_bytes))) => {
                let block_header: BlockHeader = ZcashDeserialize::zcash_deserialize(last_entry_bytes.as_ref())?;
                let arc_block_header: Arc<BlockHeader> = block_header.into();
                let block_header_hash: BlockHeaderHash = arc_block_header.as_ref().into();
//              let height_repr: HeightRepr = last_entry_height_bytes.clone().into();
//              match last_entry_height_bytes.clone().into() {
//                  HeightRepr::Int(block_height) => Ok(Some((arc_block_header, block_header_hash, block_height))),
//                  _ => Ok(None),
//              }
                if let HeightRepr::Int(block_height) = last_entry_height_bytes.clone().into() { Ok(Some((arc_block_header, block_header_hash, block_height))) } else { Ok(None) }
            },
            _ => {
                let mut error_result: String = String::from("");
//              for error in [last_entry, last_entry_height].iter() {
//                  match error {
//                      Some(Err(e)) => error_result = format!(" {:?} {:?};", error_result, e),
//                      _ => error_result = format!(" {:?};", error_result),
//                  }
//              };
                for error in &[last_entry, last_entry_height] {
                    if let Some(Err(e)) = error {
                        error_result = format!(" {:?} {:?};", error_result, e)
                    } else {
                        error_result = format!(" {:?};", error_result)
                    }
                };
                Err(format!("Error: {:?}", error_result))?
            },
        }
    }

    pub(super) fn get_height(&self, hash: BlockHeaderHash) -> Result<Option<BlockHeight>, Error> {
        let hash_height = self.storage.open_tree(b"hash_height")?;
        let key = &hash.0;
        let value = hash_height.get(key);

        match value {
            Ok(Some(vec)) => {
//              let mut bytes: [u8; 4] = [0u8; 4];
//              bytes.clone_from_slice(&vec);
//              let block_height = BlockHeight(u32::from_be_bytes(bytes));

//              let height_repr: HeightRepr = vec.into();
//              match vec.into() {
//                  HeightRepr::Int(block_height) => Ok(Some(block_height)),
//                  _ => Ok(None),
//              }

                if let HeightRepr::Int(block_height) = vec.into() { Ok(Some(block_height)) } else { Ok(None) }
            },
            Err(e) => Err(e)?,
            Ok(None) => Ok(None),
        }
    }

    fn contains(&self, hash: &BlockHeaderHash) -> Result<bool, Error> {
        let by_hash = self.storage.open_tree(b"by_hash")?;
        let key = &hash.0;

        Ok(by_hash.contains_key(key)?)
    }
}

impl Default for SledState {
    fn default() -> Self {
        let config = crate::Config::default();
        Self::new(&config)
    }
}

impl Service<RequestBlockHeader> for SledState {
    type Response = Response;
    type Error = Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RequestBlockHeader) -> Self::Future {
        match req {
            RequestBlockHeader::AddBlockHeader { block_header, block_height } => {
                let mut storage = self.clone();
                async move {
                    storage
                        .insert(block_header, block_height)
                        .map(|(hash, height)| Response::Added { hash, height })
                }
                .boxed()
            }
            RequestBlockHeader::GetBlockHeader { query } => {
                let storage = self.clone();
                async move {
                    let block_header = storage.get(query)?;
                    let hash: BlockHeaderHash = block_header.clone().unwrap().as_ref().into();
                    let height = storage.get_height(hash)?.unwrap();
                    block_header
                        .map(|block_header| Response::BlockHeader { block_header: block_header, block_height: height })
                        .ok_or_else(|| "GetBlockHeader - block header could not be found".into())
                }
                .boxed()
            }
            RequestBlockHeader::GetBlockHeight { hash } => {
                let storage = self.clone();
                async move {
                    storage
                        .get_height(hash)?
                        .map(|block_height| Response::BlockHeight { block_height })
                        .ok_or_else(|| "GetBlockHeight - block height could not be found".into())
                }
                .boxed()
            }
            RequestBlockHeader::GetTip => {
                let storage = self.clone();
                async move {
                    storage
                        .get_tip()?
                        .map(|(_header, hash, height)| Response::Tip { hash, height })
                        .ok_or_else(|| "GetTip - latest block header, which is the tip of the current best chain, couldn't be found".into())
                }
                .boxed()
            }
            RequestBlockHeader::GetDepth { hash } => {
                let storage = self.clone();

                async move {
                    if !storage.contains(&hash)? {
                        return Ok(Response::Depth(None));
                    }

                    let block_header_height = storage
                        .get_height(hash)?
                        .expect("GetDepth - block header must be present if contains() returned true");

                    let tip = storage
                        .get_tip()?
                        .map(|(_header, _hash, height)| height)
                        .expect("GetDepth - storage must have a tip if it contains() the previous block header");

                    let depth = tip.0 - block_header_height.0;

                    Ok(Response::Depth(Some(depth)))
                }
                .boxed()
            }
         /* RequestBlockHeader::GetDepth { hash: _ } => {
                async move { Ok(Response::Depth(None)) }.boxed()
            } */
        }
    }
}

use sled::IVec;

// Rust compiler's orphaning rules doesn't accept impl's for types in external crates
// Thus need to implement local type (enum) which encapsulates external types for impl's of generic traits From<T> or Into<T>
#[derive(Clone)]
enum HeightRepr {
    Int(BlockHeight),
    Byte(IVec),
}

impl From<IVec> for HeightRepr {
    fn from(vec: IVec) -> Self {
        let mut bytes: [u8; 4] = [0u8; 4];
        bytes.clone_from_slice(&vec);
        Self::Int(BlockHeight(u32::from_be_bytes(bytes)))
    }
}

impl From<BlockHeight> for HeightRepr {
    fn from(height: BlockHeight) -> Self {
        let bytes = height.0.to_be_bytes();
        Self::Byte(IVec::from(&bytes))
    }
}

/*
// Impossible implementors due to Rust compiler's orphaning rules in impl's for types in external crates

impl From<IVec> for BlockHeight {
    fn from(vec: IVec) -> Self {
        let mut bytes: [u8; 4];
        bytes.clone_from_slice(&vec);
        BlockHeight(u32::from_be_bytes(bytes))
    }
}

impl Into<BlockHeight> for IVec {
    fn into(self) -> BlockHeight {
        let mut bytes: [u8; 4];
        bytes.clone_from_slice(&self);
        BlockHeight(u32::from_be_bytes(bytes))
    }
}

impl From<BlockHeight> for IVec {
    fn from(height: BlockHeight) -> Self {
        let bytes = height.0.to_be_bytes();
        IVec::from(&bytes)
    }
}

impl Into<IVec> for BlockHeight {
    fn into(self) -> IVec {
        let bytes = self.0.to_be_bytes();
        IVec::from(&bytes)
    }
}
*/

/// An alternate repr for `BlockHeight` that implements `AsRef<[u8]>` for usage
/// with sled
struct BytesHeight(u32, [u8; 4]);

impl From<BlockHeight> for BytesHeight {
    fn from(height: BlockHeight) -> Self {
        let bytes = height.0.to_be_bytes();
        Self(height.0, bytes)
    }
}

impl AsRef<[u8]> for BytesHeight {
    fn as_ref(&self) -> &[u8] {
        &self.1[..]
    }
}

/*
pub(super) enum BlockQuery {
    ByHash(BlockHeaderHash),
    ByHeight(BlockHeight),
}

impl From<BlockHeaderHash> for BlockQuery {
    fn from(hash: BlockHeaderHash) -> Self {
        Self::ByHash(hash)
    }
}

impl From<BlockHeight> for BlockQuery {
    fn from(height: BlockHeight) -> Self {
        Self::ByHeight(height)
    }
}
*/

/// Return's a type that implement's the `zebra_state::Service` using `sled`
pub fn init(
    config: Config,
) -> impl Service<
    RequestBlockHeader,
    Response = Response,
    Error = Error,
    Future = impl Future<Output = Result<Response, Error>>,
> + Sync
  + Send
  + Clone
  + 'static {
    Buffer::new(SledState::new(&config), 1)
}
