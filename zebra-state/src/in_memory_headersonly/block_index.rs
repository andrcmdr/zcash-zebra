use std::{
    collections::{
//      btree_map::Entry as BTreeMapEntry,
        hash_map::Entry as HashMapEntry,
        BTreeMap,
        HashMap
    },
    error,
    sync::Arc,
};

type Error = Box<dyn error::Error + Send + Sync + 'static>;

use zebra_chain::{
    block::{BlockHeader, BlockHeaderHash},
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

impl BlockIndex<BlockHeader> {
    pub fn insert(
        &mut self,
        block_header: impl Into<Arc<BlockHeader>>,
    ) -> Result<BlockHeaderHash, Error> {
        let block_header = block_header.into();
        let hash = block_header.as_ref().into();
//      let height = block.coinbase_height().unwrap(); // BlockIndex::<BlockHeader>{ by_height } isn't applicable for headers handling

        match self.by_hash.entry(hash) {
            HashMapEntry::Vacant(_entry) => {
             // let _ = _entry.insert(block_header.clone()); // prevent writing to the same key/entry in the same HashMap (BlockIndex::<BlockHeader>{ by_hash }), because of BlockIndex::<BlockHeader>{ by_height } isn't applicable for headers handling - thus comment this line to prevent double write
                let _ = self.by_hash.insert(hash, block_header);
                Ok(hash)
            }
            HashMapEntry::Occupied(_) => Err("forks in the chain aren't supported yet")?,
        }
    }

    pub fn get(&self, query: impl Into<BlockQuery>) -> Result<Option<Arc<BlockHeader>>, Error> {
        let value = match query.into() {
            BlockQuery::ByHash(hash) => self.by_hash.get(&hash),
            // didn't applicable for headers handling
            BlockQuery::ByHeight(height) => self.by_height.get(&height),
        }
        .cloned();

        match value {
            Some(block_header) => Ok(Some(block_header)),
            None => Ok(None)
        }
    }

    // didn't applicable for headers handling
    pub fn get_tip(&self) -> Result<Option<Arc<BlockHeader>>, Error> {
        let last_entry = self.by_height
            .iter()
            .next_back()
            .map(|(_key, value)| value)
            .map(|block_header| block_header.clone());
//          .map(|block_header| block_header.as_ref().into()); // -> Option<BlockHeaderHash>

        match last_entry {
            Some(block_header) => Ok(Some(block_header)),
            None => Ok(None)
        }
    }

    #[allow(dead_code)]
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
