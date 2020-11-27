use zebra_chain::{
    block::{
//      Block,
        BlockHeader,
        BlockHeaderHash,
    },
    types::BlockHeight,
};
use zebra_state::{
    QueryType,
//  RequestBlock,
    RequestBlockHeader,
    Response,
};
use std::{
    error,
    future::Future,
    pin::Pin,
    sync::Arc,
};
use futures_util::FutureExt;
use tower::{Service, ServiceExt};

use crate::prelude::*;

type Error = Box<dyn error::Error + Send + Sync + 'static>;

/*
impl Default for IBCItems<BlockHeaderHash, BlockHeight> {
    fn default() -> Self {
        Self {
            hash: BlockHeaderHash([0u8; 32]),
            height: BlockHeight(0),
        }
    }
}

struct Hash(BlockHeaderHash);

impl Default for Hash {
    fn default() -> Self {
        Self(BlockHeaderHash([0u8; 32]))
    }
}

struct Height(BlockHeight);

impl Default for Height {
    fn default() -> Self {
        Self(BlockHeight(0))
    }
}
*/

impl From<BlockHeaderHash> for IBCQuery<BlockHeaderHash, BlockHeight> {
    fn from(hash: BlockHeaderHash) -> Self {
        Self::ByHash(hash)
    }
}

impl From<BlockHeight> for IBCQuery<BlockHeaderHash, BlockHeight> {
    fn from(height: BlockHeight) -> Self {
        Self::ByHeight(height)
    }
}

impl<S> Storage<S>
where
    S: Service<RequestBlockHeader, Response = Response, Error = Error> + Send + Clone + 'static,
    S::Future: Send,
    Self: Send + Sync + 'static,
{
    pub fn new(state: S) -> Self {
        Self {
            state,
        }
    }
}

impl<S> IBCRequest<BlockHeaderHash, BlockHeight> for Storage<S>
where
    S: Service<RequestBlockHeader, Response = Response, Error = Error> + Send + Clone + 'static,
    S::Future: Send,
    Self: Send + Sync + 'static,
{
//  type BlockResponse = Pin<Box<dyn Future<Output = Result<Option<Arc<Block>>, Error>> + Send + 'static>>;
//  type HeaderResponse = Pin<Box<dyn Future<Output = Result<Option<Arc<BlockHeader>>, Error>> + Send + 'static>>;
//  type HashResponse = Pin<Box<dyn Future<Output = Result<Option<BlockHeaderHash>, Error>> + Send + 'static>>;
//  type HeightResponse = Pin<Box<dyn Future<Output = Result<Option<BlockHeight>, Error>> + Send + 'static>>;
    type HeaderHeightResponse = Pin<Box<dyn Future<Output = Result<Option<(Arc<BlockHeader>, BlockHeight)>, Error>> + Send + 'static>>;
    type HashHeightResponse = Pin<Box<dyn Future<Output = Result<Option<(BlockHeaderHash, BlockHeight)>, Error>> + Send + 'static>>;

    fn get(&self, query: impl Into<IBCQuery<BlockHeaderHash, BlockHeight>>) -> Self::HeaderHeightResponse {
        let value = match query.into() {
            IBCQuery::ByHash(hash) => {
                let mut state = self.state.clone();
                async move {
                    let get_block_header = state
                    .ready_and()
                    .await?
                    .call(RequestBlockHeader::GetBlockHeader { query: QueryType::ByHash(hash) });

                    tracing::info!("Block header with hash {:?} requested!", hash);

                    match get_block_header.await? {
                        zebra_state::Response::BlockHeader { block_header, block_height } => Ok(Some((block_header, block_height))),
                        _ => Err("block header couldn't be found - either still syncing, or out of range".into()),
                    }
                }.boxed()
            }
            IBCQuery::ByHeight(height) => {
                let mut state = self.state.clone();
                async move {
                    let get_block_header = state
                    .ready_and()
                    .await?
                    .call(RequestBlockHeader::GetBlockHeader { query: QueryType::ByHeight(height) });

                    tracing::info!("Block header with height {:?} requested!", height);

                    match get_block_header.await? {
                        zebra_state::Response::BlockHeader { block_header, block_height } => Ok(Some((block_header, block_height))),
                        _ => Err("block header couldn't be found - either still syncing, or out of range".into()),
                    }
                }.boxed()
            }
        };
        value
    }

    fn get_tip(&self) -> Self::HashHeightResponse {
        let mut state = self.state.clone();
        async move {
            let get_tip = state
            .ready_and()
            .await?
            .call(RequestBlockHeader::GetTip);

            tracing::info!("Tip requested!");

            match get_tip.await? {
                Response::Tip { hash, height } => Ok(Some((hash, height))),
                _ => Err("Some error in requesting block header that is the tip of the current chain".into()),
            }
        }.boxed()
    }
}
