//! A basic implementation of the zebra-state service entirely in memory
//!
//! This service is provided as an independent implementation of the
//! zebra-state service to use in verifying the correctness of `on_disk`'s
//! `Service` implementation.
// use crate::in_memory::block_index::Index;
use super::{RequestBlockHeader, Response};
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
    block::{BlockHeader, BlockHeaderHash},
    types::BlockHeight,
};

mod block_index;

type Error = Box<dyn error::Error + Send + Sync + 'static>;

#[derive(Default)]
struct InMemoryState<T> {
    index: block_index::BlockIndex<T>,
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
                    .get(hash).expect("GetBlockHeader by hash - block header could not be found")
                    .map(|block_header| Response::BlockHeader { block_header })
                    .ok_or_else(|| "GetBlockHeader by hash - block header could not be found".into());

                async move { result }.boxed()
            }
            // didn't applicable for headers handling
            RequestBlockHeader::GetTip => {
                let result = self
                    .index
                    .get_tip().expect("GetTip - zebra-state contains no block headers")
                    .map(|block_header| block_header.as_ref().into())
                    .map(|hash| Response::Tip { hash })
                    .ok_or_else(|| "GetTip - zebra-state contains no block headers".into());

                async move { result }.boxed()
            }
            // didn't applicable for headers handling
            /* RequestBlockHeader::GetDepth { hash } => {
                let storage = self.index.clone();

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

/// Return's a type that implement's the `zebra_state::Service` entirely in
/// memory using `HashMaps`
// pub fn init<T: Sync + Send + Clone + Copy + 'static>() -> impl Service<
pub fn init() -> impl Service<
    RequestBlockHeader,
    Response = Response,
    Error = Error,
    Future = impl Future<Output = Result<Response, Error>>,
> + Sync
  + Send
  + Clone
  + 'static {
    Buffer::new(InMemoryState::<BlockHeader>{
        index: block_index::BlockIndex::<BlockHeader>{
            by_hash: HashMap::<BlockHeaderHash, Arc<BlockHeader>>::default(),
            by_height: BTreeMap::<BlockHeight, Arc<BlockHeader>>::default(),
        },
    }, 1)
}
