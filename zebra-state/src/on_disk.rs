//! The primary implementation of the `zebra_state::Service` built upon sled
use super::{RequestBlock, Response, QueryType};
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
    block::{Block, BlockHeaderHash},
    types::BlockHeight,
};

type Error = Box<dyn error::Error + Send + Sync + 'static>;

#[derive(Clone)]
struct SledState {
    storage: sled::Db,
}

impl SledState {
    pub(crate) fn new(config: &Config) -> Self {
        let config = config.sled_config("blocks");
        Self {
            storage: config.open().unwrap(),
        }
    }

    pub(super) fn insert(
        &mut self,
        block: impl Into<Arc<Block>>,
    ) -> Result<(BlockHeaderHash, BlockHeight), Error> {
        let block = block.into();
        let hash: BlockHeaderHash = block.as_ref().into();
        let height = block.coinbase_height().unwrap();

        let by_height = self.storage.open_tree(b"by_height")?;
        let by_hash = self.storage.open_tree(b"by_hash")?;

        let mut bytes = Vec::new();
        block.zcash_serialize(&mut bytes)?;

        // TODO(jlusby): make this transactional
        by_height.insert(&height.0.to_be_bytes(), bytes.as_slice())?;
        by_hash.insert(&hash.0, bytes)?;

        Ok((hash, height))
    }

    pub(super) fn get(&self, query: impl Into<QueryType>) -> Result<Option<Arc<Block>>, Error> {
        let query = query.into();
        let value = match query {
            QueryType::ByHash(hash) => {
                let by_hash = self.storage.open_tree(b"by_hash")?;
                let key = &hash.0;
                by_hash.get(key)?
            }
            QueryType::ByHeight(height) => {
                let by_height = self.storage.open_tree(b"by_height")?;
                let key = height.0.to_be_bytes();
                by_height.get(key)?
            }
        };

        if let Some(bytes) = value {
            let bytes = bytes.as_ref();
            let block = ZcashDeserialize::zcash_deserialize(bytes)?;
            Ok(Some(block))
        } else {
            Ok(None)
        }
    }

    pub(super) fn get_tip(&self) -> Result<Option<Arc<Block>>, Error> {
        let tree = self.storage.open_tree(b"by_height")?;
        let last_entry = tree.iter().values().next_back();

        match last_entry {
            Some(Ok(bytes)) => Ok(Some(ZcashDeserialize::zcash_deserialize(bytes.as_ref())?)),
            Some(Err(e)) => Err(e)?,
            None => Ok(None),
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

impl Service<RequestBlock> for SledState {
    type Response = Response;
    type Error = Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RequestBlock) -> Self::Future {
        match req {
            RequestBlock::AddBlock { block } => {
                let mut storage = self.clone();
                async move {
                    storage
                        .insert(block)
                        .map(|(hash, height)| Response::Added { hash, height })
                }
                .boxed()
            }
            RequestBlock::GetBlock { query } => {
                let storage = self.clone();
                async move {
                    storage
                        .get(query)?
                        .map(|block| Response::Block { block })
                        .ok_or_else(|| "GetBlock - block could not be found".into())
                }
                .boxed()
            }
            RequestBlock::GetBlockHeight { hash } => {
                let storage = self.clone();
                async move {
                    storage
                        .get(hash)?
                        .map(|block| block.coinbase_height().unwrap())
                        .map(|block_height| Response::BlockHeight { block_height })
                        .ok_or_else(|| "GetBlockHeight - block height could not be found".into())
                }
                .boxed()
            }
            RequestBlock::GetTip => {
                let storage = self.clone();
                async move {
                    storage
                        .get_tip()?
                        .map(|block| (block.as_ref().into(), block.coinbase_height().unwrap()))
                        .map(|(hash, height)| Response::Tip { hash, height })
                        .ok_or_else(|| "GetTip - latest block, which is the tip of the current best chain, couldn't be found".into())
                }
                .boxed()
            }
            RequestBlock::GetDepth { hash } => {
                let storage = self.clone();

                async move {
                    if !storage.contains(&hash)? {
                        return Ok(Response::Depth(None));
                    }

                    let block = storage
                        .get(hash)?
                        .expect("GetDepth - block must be present if contains() returned true");

                    let tip = storage
                        .get_tip()?
                        .expect("GetDepth - storage must have a tip if it contains() the previous block");

                    let depth =
                        tip.coinbase_height().unwrap().0 - block.coinbase_height().unwrap().0;

                    Ok(Response::Depth(Some(depth)))
                }
                .boxed()
            }
        }
    }
}

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
    RequestBlock,
    Response = Response,
    Error = Error,
    Future = impl Future<Output = Result<Response, Error>>,
> + Sync
  + Send
  + Clone
  + 'static {
    Buffer::new(SledState::new(&config), 1)
}
