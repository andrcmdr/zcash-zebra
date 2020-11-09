//! Application-local prelude: conveniently describes types/traits/functions/macros
//! which are generally/commonly useful and should be available everywhere.

use std::{
    error,
//  sync::Arc,
};

type Error = Box<dyn error::Error + Send + Sync + 'static>;

pub(super) trait IBCRunnable {
    /// Run this `Runnable`
    fn run(&self);
}

pub(super) trait IBCRequest<Hash, Height> {
    type BlockResponse;
    type HeaderResponse;
    type HashResponse;
    type HeightResponse;
    type HashHeightResponse;
//  fn get(&self, query: impl Into<IBCQuery<Hash, Height>>) -> Result<Option<Arc<BlockORHeader>>, Error>;
//  fn get_tip(&self) -> Result<Option<Arc<Hash>>, Error>;
    fn get(&self, query: impl Into<IBCQuery<Hash, Height>>) -> Self::HeaderResponse;
    fn get_tip(&self) -> Self::HashHeightResponse;
}

#[derive(Default, Clone)]
pub(super) struct IBCItems<Hash, Height> {
    pub hash: Hash,
    pub height: Height,
}

pub(super) enum IBCQuery<Hash, Height> {
    ByHash(Hash),
    ByHeight(Height),
}
