//! Application-local prelude: conveniently describes types/traits/functions/macros
//! which are generally/commonly useful and should be available everywhere.

use std::{
    error,
//  sync::Arc,
};

type Error = Box<dyn error::Error + Send + Sync + 'static>;

#[derive(Clone, Copy)]
pub struct IBCStorage<S>
where
    Self: Send + Sync + 'static,
{
    pub state: S,
}

#[derive(Clone, Copy)]
pub enum IBCQuery<Hash, Height> {
    ByHash(Hash),
    ByHeight(Height),
}

pub trait IBCRequest<Hash, Height> {
//  type BlockResponse;
//  type HeaderResponse;
//  type HashResponse;
//  type HeightResponse;
    type HeaderHeightResponse;
    type HashHeightResponse;
    fn get(&self, query: impl Into<IBCQuery<Hash, Height>>) -> Self::HeaderHeightResponse;
    fn get_tip(&self) -> Self::HashHeightResponse;
}

/*
#[derive(Clone, Copy)]
pub struct IBCItems<Hash, Height> {
    pub hash: Hash,
    pub height: Height,
}
*/
