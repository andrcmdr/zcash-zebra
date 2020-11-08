//! A basic implementation of the zebra-state service entirely in memory
//!
//! This service is provided as an independent implementation of the
//! zebra-state service to use in verifying the correctness of `on_disk`'s
//! `Service` implementation.
use super::{RequestBlockHeader, Response, QueryType};
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
            RequestBlockHeader::AddBlockHeader { block_header, block_height } => {
                let result = self
                    .index
                    .insert(block_header, block_height)
                    .map(|(hash, height)| Response::Added { hash, height });

                async move { result }.boxed()
            }
            RequestBlockHeader::GetBlockHeader { query } => {
                let result = self
                    .index
                    .get(query).unwrap() //? .unwrap_or(default: T) .expect("GetBlockHeader - block header could not be found")
                    .map(|block_header| Response::BlockHeader { block_header })
                    .ok_or_else(|| "GetBlockHeader - block header could not be found".into());

                async move { result }.boxed()
            }
            RequestBlockHeader::GetTip => {
                let result = self
                    .index
                    .get_tip().unwrap() //? .unwrap_or(default: T) .expect("GetTip - zebra-state contains no block headers")
                    .map(|(_header, hash, height)| Response::Tip { hash, height })
                    .ok_or_else(|| "GetTip - zebra-state contains no block headers".into());

                async move { result }.boxed()
            }
            RequestBlockHeader::GetDepth { hash } => {
                let storage = self.index.clone();

                async move {
                    if !storage.contains(&hash)? {
                        return Ok(Response::Depth(None));
                    }

                    let block_header_height = storage
                        .get_height(hash)?
                        .expect("block header must be present if contains() returned true");

                    let tip = storage
                        .get_tip()?
                        .map(|(_header, _hash, height)| height)
                        .expect("storage must have a tip if it contains() the previous block header");

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

/// Return's a type that implement's the `zebra_state::Service` entirely in
/// memory using `HashMaps`
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
            hash_height: HashMap::<BlockHeaderHash, BlockHeight>::default(),
        },
    }, 1)
}
