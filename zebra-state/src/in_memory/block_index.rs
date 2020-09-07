use std::{
    collections::{
        btree_map::Entry as BTreeMapEntry,
//      hash_map::Entry as HashMapEntry,
        BTreeMap,
        HashMap
    },
    error,
    sync::Arc,
};

type Error = Box<dyn error::Error + Send + Sync + 'static>;

use zebra_chain::{
    block::{Block, BlockHeaderHash},
    types::BlockHeight,
};

/*
pub(super) trait Index<T> {
    fn insert(&mut self, block_item: impl Into<Arc<T>>) -> Result<BlockHeaderHash, Box<dyn Error + Send + Sync + 'static>>;
    fn get(&mut self, query: impl Into<BlockQuery>) -> Option<Arc<T>>;
    fn get_tip(&self) -> Option<BlockHeaderHash>;
}
*/

#[derive(Default, Clone)]
pub(super) struct BlockIndex<T> {
    pub by_hash: HashMap<BlockHeaderHash, Arc<T>>,
    pub by_height: BTreeMap<BlockHeight, Arc<T>>,
}

impl BlockIndex<Block> {
    pub fn insert(
        &mut self,
        block: impl Into<Arc<Block>>,
    ) -> Result<BlockHeaderHash, Error> {
        let block = block.into();
        let hash = block.as_ref().into();
        let height = block.coinbase_height().unwrap();

        match self.by_height.entry(height) {
            BTreeMapEntry::Vacant(entry) => {
                let _ = entry.insert(block.clone());
                let _ = self.by_hash.insert(hash, block);
                Ok(hash)
            }
            BTreeMapEntry::Occupied(_) => Err("forks in the chain aren't supported yet")?,
        }
    }

    pub fn get(&self, query: impl Into<BlockQuery>) -> Result<Option<Arc<Block>>, Error> {
        let value = match query.into() {
            BlockQuery::ByHash(hash) => self.by_hash.get(&hash),
            BlockQuery::ByHeight(height) => self.by_height.get(&height),
        }
        .cloned();

        match value {
            Some(block) => Ok(Some(block)),
            None => Ok(None)
        }
    }

    pub fn get_tip(&self) -> Result<Option<Arc<Block>>, Error> {
        let last_entry = self.by_height
            .iter()
            .next_back()
            .map(|(_key, value)| value)
            .map(|block| block.clone());
//          .map(|block| block.as_ref().into()); // -> Option<BlockHeaderHash>

        match last_entry {
            Some(block) => Ok(Some(block)),
            None => Ok(None)
        }
    }

    pub fn contains(&self, hash: &BlockHeaderHash) -> Result<bool, Error> {
        let key = &hash;
        Ok(self.by_hash.contains_key(key))
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
