//! A basic implementation of the zebra-state service entirely in memory
//!
//! This service is provided as an independent implementation of the
//! zebra-state service to use in verifying the correctness of `on_disk`'s
//! `Service` implementation.
// use crate::in_memory::block_index::Index;
use super::{RequestBlock, RequestBlockHeader, Response};
use futures::prelude::*;
use std::{
    collections::{BTreeMap, HashMap},
    error,
    sync::Arc,
    future::Future,
    pin::Pin,
    task::{Context, Poll},
};
use tower::{buffer::Buffer, Service};
use zebra_chain::{
//  block::BlockHeaderHash,
    block::{BlockHeaderHash, Block, BlockHeader},
    types::BlockHeight,
};

mod block_index;

#[derive(Default)]
struct InMemoryState<T> {
    index: block_index::BlockIndex<T>,
}

impl<T> InMemoryState<T> {
    fn contains(&mut self, _hash: BlockHeaderHash) -> Result<Option<u32>, Error> {
        todo!()
    }
}

impl Service<RequestBlock> for InMemoryState<Block> {
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
                let result = self
                    .index
                    .insert(block)
                    .map(|hash| Response::Added { hash });

                async { result }.boxed()
            }
            RequestBlock::GetBlock { hash } => {
                let result = self
                    .index
                    .get(hash)
                    .map(|block| Response::Block { block })
                    .ok_or_else(|| "block could not be found".into());

                async move { result }.boxed()
            }
            RequestBlock::GetTip => {
                let result = self
                    .index
                    .get_tip()
                    .map(|hash| Response::Tip { hash })
                    .ok_or_else(|| "zebra-state contains no blocks".into());

                async move { result }.boxed()
            }
            RequestBlock::GetDepth { hash } => {
                let res = self.contains(hash);

                async move {
                    let depth = res?;

                    Ok(Response::Depth(depth))
                }
                .boxed()
            }
        }
    }
}

impl Service<RequestBlockHeader> for InMemoryState<BlockHeader> {
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
                let result = self
                    .index
                    .insert(block_header)
                    .map(|hash| Response::Added { hash });

                async { result }.boxed()
            }
            RequestBlockHeader::GetBlockHeader { hash } => {
                let result = self
                    .index
                    .get(hash)
                    .map(|block_header| Response::BlockHeader { block_header })
                    .ok_or_else(|| "block header could not be found".into());

                async move { result }.boxed()
            }
            RequestBlockHeader::GetTip => {
                let result = self
                    .index
                    .get_tip()
                    .map(|hash| Response::Tip { hash })
                    .ok_or_else(|| "zebra-state contains no block headers".into());

                async move { result }.boxed()
            }
            RequestBlockHeader::GetDepth { hash } => {
                let res = self.contains(hash);

                async move {
                    let depth = res?;

                    Ok(Response::Depth(depth))
                }
                .boxed()
            }
        }
    }
}

/// Return's a type that implement's the `zebra_state::Service` entirely in
/// memory using `HashMaps`
pub fn init<T: Sync + Send + 'static>() -> impl Service<
    RequestBlockHeader,
    Response = Response,
    Error = Error,
    Future = impl Future<Output = Result<Response, Error>>,
> + Send
  + Clone
  + 'static {
    Buffer::new(InMemoryState::<BlockHeader>{
        index: block_index::BlockIndex::<BlockHeader>{
            by_hash: HashMap::<BlockHeaderHash, Arc<BlockHeader>>::default(),
            by_height: BTreeMap::<BlockHeight, Arc<BlockHeader>>::default(),
        },
    }, 1)
}

type Error = Box<dyn error::Error + Send + Sync + 'static>;
