//! A basic implementation of the zebra-state service entirely in memory
//!
//! This service is provided as an independent implementation of the
//! zebra-state service to use in verifying the correctness of `on_disk`'s
//! `Service` implementation.
use super::{RequestBlock, Response, QueryType};
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
    block::{Block, BlockHeaderHash},
    types::BlockHeight,
};

mod block_index;

type Error = Box<dyn error::Error + Send + Sync + 'static>;

#[derive(Default)]
struct InMemoryState<T> {
    index: block_index::BlockIndex<T>,
}

impl<T: Into<QueryType>> Service<RequestBlock<T>> for InMemoryState<Block> {
    type Response = Response;
    type Error = Error;
    type Future =
        Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>> + Send + 'static>>;

    fn poll_ready(&mut self, _: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        Poll::Ready(Ok(()))
    }

    fn call(&mut self, req: RequestBlock<T>) -> Self::Future {
        match req {
            RequestBlock::AddBlock { block } => {
                let result = self
                    .index
                    .insert(block)
                    .map(|(hash, height)| Response::Added { hash, height });

                async move { result }.boxed()
            }
            RequestBlock::GetBlock { query } => {
                let result = self
                    .index
                    .get(query).unwrap() //? .unwrap_or(default: T) .expect("GetBlock - block could not be found")
                    .map(|block| Response::Block { block })
                    .ok_or_else(|| "GetBlock - block could not be found".into());

                async move { result }.boxed()
            }
            RequestBlock::GetTip => {
                let result = self
                    .index
                    .get_tip().unwrap() //? .unwrap_or(default: T) .expect("GetTip - zebra-state contains no blocks")
                    .map(|block| (block.as_ref().into(), block.coinbase_height().unwrap()))
                    .map(|(hash, height)| Response::Tip { hash, height })
                    .ok_or_else(|| "GetTip - zebra-state contains no blocks".into());

                async move { result }.boxed()
            }
            RequestBlock::GetDepth { hash } => {
                let storage = self.index.clone();

                async move {
                    if !storage.contains(&hash)? {
                        return Ok(Response::Depth(None));
                    }

                    let block = storage
                        .get(hash)?
                        .expect("block must be present if contains() returned true");

                    let tip = storage
                        .get_tip()?
                        .expect("storage must have a tip if it contains() the previous block");

                    let depth =
                        tip.coinbase_height().unwrap().0 - block.coinbase_height().unwrap().0;

                    Ok(Response::Depth(Some(depth)))
                }
                .boxed()
            }
        }
    }
}

/// Return's a type that implement's the `zebra_state::Service` entirely in
/// memory using `HashMaps`
pub fn init<T: Into<QueryType> + Send + Sync + Clone + 'static>() -> impl Service<
    RequestBlock<T>,
    Response = Response,
    Error = Error,
    Future = impl Future<Output = Result<Response, Error>>,
> + Sync
  + Send
  + Clone
  + 'static {
    Buffer::new(InMemoryState::<Block>{
        index: block_index::BlockIndex::<Block>{
            by_hash: HashMap::<BlockHeaderHash, Arc<Block>>::default(),
            by_height: BTreeMap::<BlockHeight, Arc<Block>>::default(),
        },
    }, 1)
}
