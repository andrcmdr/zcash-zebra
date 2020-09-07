//! The primary implementation of the `zebra_state::Service` built upon sled
use super::{RequestBlockHeader, Response};
use crate::Config;
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

/*
pub(super) trait State<T> {
    fn new(config: &Config) -> Self;
    fn insert(&mut self, block_item: impl Into<Arc<T>>) -> Result<BlockHeaderHash, Error>;
    fn get(&self, query: impl Into<BlockQuery>) -> Result<Option<Arc<T>>, Error>;
    fn get_tip(&self) -> Result<Option<Arc<T>>, Error>;
}
*/

#[derive(Clone)]
struct SledState {
    storage: sled::Db,
}

impl SledState {
    pub(crate) fn new(config: &Config) -> Self {
        let config = config.sled_config();

        Self {
            storage: config.open().unwrap(),
        }
    }

    pub(super) fn insert(
        &mut self,
        block_header: impl Into<Arc<BlockHeader>>,
    ) -> Result<BlockHeaderHash, Error> {
        let block_header = block_header.into();
        let hash: BlockHeaderHash = block_header.as_ref().into();
//      let height = block.coinbase_height().unwrap();

//      let by_height = self.storage.open_tree(b"by_height")?;
        let by_hash = self.storage.open_tree(b"by_hash")?;

        let mut bytes = Vec::new();
        block_header.zcash_serialize(&mut bytes)?;

        // TODO(jlusby): make this transactional
//      by_height.insert(&height.0.to_be_bytes(), bytes.as_slice())?;
        by_hash.insert(&hash.0, bytes)?;

        Ok(hash)
    }

    pub(super) fn get(&self, query: impl Into<BlockQuery>) -> Result<Option<Arc<BlockHeader>>, Error> {
        let query = query.into();
        let value = match query {
            BlockQuery::ByHash(hash) => {
                let by_hash = self.storage.open_tree(b"by_hash")?;
                let key = &hash.0;
                by_hash.get(key)?
            }
            // didn't applicable for headers handling
            BlockQuery::ByHeight(height) => {
                let by_height = self.storage.open_tree(b"by_height")?;
                let key = height.0.to_be_bytes();
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

    // didn't applicable for headers handling
    pub(super) fn get_tip(&self) -> Result<Option<Arc<BlockHeader>>, Error> {
        let tree = self.storage.open_tree(b"by_height")?;
        let last_entry = tree.iter().values().next_back();

        match last_entry {
            Some(Ok(bytes)) => Ok(Some(ZcashDeserialize::zcash_deserialize(bytes.as_ref())?)),
            Some(Err(e)) => Err(e)?,
            None => Ok(None),
        }
    }

    #[allow(dead_code)]
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
            RequestBlockHeader::AddBlockHeader { block_header } => {
                let mut storage = self.clone();

                async move { storage.insert(block_header).map(|hash| Response::Added { hash }) }.boxed()
            }
            RequestBlockHeader::GetBlockHeader { hash } => {
                let storage = self.clone();
                async move {
                    storage
                        .get(hash)?
                        .map(|block_header| Response::BlockHeader { block_header })
                        .ok_or_else(|| "block header could not be found".into())
                }
                .boxed()
            }
            // didn't applicable for headers handling
            RequestBlockHeader::GetTip => {
                let storage = self.clone();
                async move {
                    storage
                        .get_tip()?
                        .map(|block_header| block_header.as_ref().into())
                        .map(|hash| Response::Tip { hash })
                        .ok_or_else(|| "zebra-state contains no block headers".into())
                }
                .boxed()
            }
            // didn't applicable for headers handling
            /* RequestBlockHeader::GetDepth { hash } => {
                let storage = self.clone();

                async move {
                    if !storage.contains(&hash)? {
                        return Ok(Response::Depth(None));
                    }

                    let block_header = storage
                        .get(hash)?
                        .expect("block header must be present if contains() returned true");

                    let tip = storage
                        .get_tip()?
                        .expect("storage must have a tip if it contains() the previous block header");

                    let depth =
                        tip.coinbase_height().unwrap().0 - block_header.coinbase_height().unwrap().0;

                    Ok(Response::Depth(Some(depth)))
                }
                .boxed()
            } */
            RequestBlockHeader::GetDepth { hash: _ } => {
                async move { Ok(Response::Depth(None)) }.boxed()
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

/// Return's a type that implement's the `zebra_state::Service` using `sled`
pub fn init(
    config: Config,
) -> impl Service<
    RequestBlockHeader,
    Response = Response,
    Error = Error,
    Future = impl Future<Output = Result<Response, Error>>,
> + Send
  + Clone
  + 'static {
    Buffer::new(SledState::new(&config), 1)
}
