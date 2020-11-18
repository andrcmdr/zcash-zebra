//! Application-local prelude: conveniently describes types/traits/functions/macros
//! which are generally/commonly useful and should be available everywhere.

use std::path::{
//  Path,
    PathBuf,
};

use std::{
    error,
//  sync::Arc,
};

type Error = Box<dyn error::Error + Send + Sync + 'static>;

#[derive(Clone, Debug)]
pub struct Config {
    pub path: PathBuf,
}

pub trait IBCRunnable {
    /// Run this `Runnable`
    fn run(&self, config_file_path: Option<PathBuf>);
}

pub trait IBCRequest<Hash, Height> {
    type BlockResponse;
    type HeaderResponse;
    type HashResponse;
    type HeightResponse;
    type HeaderHeightResponse;
    type HashHeightResponse;
//  fn get(&self, query: impl Into<IBCQuery<Hash, Height>>) -> Result<Option<Arc<BlockORHeader>>, Error>;
//  fn get_tip(&self) -> Result<Option<Arc<Hash>>, Error>;
    fn get(&self, query: impl Into<IBCQuery<Hash, Height>>) -> Self::HeaderHeightResponse;
    fn get_tip(&self) -> Self::HashHeightResponse;
}

#[derive(Clone)]
pub struct IBCItems<Hash, Height> {
    pub hash: Hash,
    pub height: Height,
}

#[derive(Clone)]
pub enum IBCQuery<Hash, Height> {
    ByHash(Hash),
    ByHeight(Height),
}
