use std::{
    collections::{btree_map::Entry, BTreeMap, HashMap},
    error::Error,
    sync::Arc,
};
use zebra_chain::{
    block::{Block, BlockHeaderHash, BlockHeader},
    types::BlockHeight,
};

#[derive(Default)]
pub(super) struct BlockIndex<T> {
    by_hash: HashMap<BlockHeaderHash, Arc<T>>,
    by_height: BTreeMap<BlockHeight, Arc<T>>,
}

impl BlockIndex<Block> {
    pub(super) fn insert(
        &mut self,
        block: impl Into<Arc<Block>>,
    ) -> Result<BlockHeaderHash, Box<dyn Error + Send + Sync + 'static>> {
        let block = block.into();
        let hash = block.as_ref().into();
        let height = block.coinbase_height().unwrap();

        match self.by_height.entry(height) {
            Entry::Vacant(entry) => {
                let _ = entry.insert(block.clone());
                let _ = self.by_hash.insert(hash, block);
                Ok(hash)
            }
            Entry::Occupied(_) => Err("forks in the chain aren't supported yet")?,
        }
    }

    pub(super) fn get(&mut self, query: impl Into<BlockQuery>) -> Option<Arc<Block>> {
        match query.into() {
            BlockQuery::ByHash(hash) => self.by_hash.get(&hash),
            BlockQuery::ByHeight(height) => self.by_height.get(&height),
        }
        .cloned()
    }

    pub(super) fn get_tip(&self) -> Option<BlockHeaderHash> {
        self.by_height
            .iter()
            .next_back()
            .map(|(_key, value)| value)
            .map(|block| block.as_ref().into())
    }
}

impl BlockIndex<BlockHeader> {
    pub(super) fn get_header(&mut self, query: impl Into<BlockQuery>) -> Option<Arc<BlockHeader>> {
        match query.into() {
            BlockQuery::ByHash(hash) => self.by_hash.get(&hash),
            BlockQuery::ByHeight(height) => self.by_height.get(&height),
        }
        .cloned()
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
