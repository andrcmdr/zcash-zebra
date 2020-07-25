use std::{
    collections::{
        btree_map::Entry as BTreeMapEntry,
        hash_map::Entry as HashMapEntry,
        BTreeMap,
        HashMap
    },
    error::Error,
    sync::Arc,
};
use zebra_chain::{
    block::{BlockHeaderHash, Block, BlockHeader},
    types::BlockHeight,
};

#[derive(Default)]
pub(super) struct BlockIndex<T> {
    by_hash: HashMap<BlockHeaderHash, Arc<T>>,
    by_height: BTreeMap<BlockHeight, Arc<T>>,
}

pub(super) trait Index<T> {
    fn insert(&mut self, block: impl Into<Arc<T>>) -> Result<BlockHeaderHash, Box<dyn Error + Send + Sync + 'static>>;
    fn get(&mut self, query: impl Into<BlockQuery>) -> Option<Arc<T>>;
    fn get_tip(&self) -> Option<BlockHeaderHash>;
}

impl Index<Block> for BlockIndex<Block> {
    fn insert(
        &mut self,
        block: impl Into<Arc<Block>>,
    ) -> Result<BlockHeaderHash, Box<dyn Error + Send + Sync + 'static>> {
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

    fn get(&mut self, query: impl Into<BlockQuery>) -> Option<Arc<Block>> {
        match query.into() {
            BlockQuery::ByHash(hash) => self.by_hash.get(&hash),
            BlockQuery::ByHeight(height) => self.by_height.get(&height),
        }
        .cloned()
    }

    fn get_tip(&self) -> Option<BlockHeaderHash> {
        self.by_height
            .iter()
            .next_back()
            .map(|(_key, value)| value)
            .map(|block| block.as_ref().into())
    }
}

impl Index<BlockHeader> for BlockIndex<BlockHeader> {
    fn insert(
        &mut self,
        block_header: impl Into<Arc<BlockHeader>>,
    ) -> Result<BlockHeaderHash, Box<dyn Error + Send + Sync + 'static>> {
        let block_header = block_header.into();
        let hash = block_header.as_ref().into();
//      let height = block_header.coinbase_height().unwrap(); // BlockIndex::<BlockHeader>{ by_height } is unusable

        match self.by_hash.entry(hash) {
            HashMapEntry::Vacant(entry) => {
             // let _ = entry.insert(block_header.clone()); // write to the same key/entry in the same HashMap (BlockIndex::<BlockHeader>{ by_hash }) - thus comment this string to prevent double write
                let _ = self.by_hash.insert(hash, block_header);
                Ok(hash)
            }
            HashMapEntry::Occupied(_) => Err("forks in the chain aren't supported yet")?,
        }
    }

    fn get(&mut self, query: impl Into<BlockQuery>) -> Option<Arc<BlockHeader>> {
        match query.into() {
            BlockQuery::ByHash(hash) => self.by_hash.get(&hash),
            BlockQuery::ByHeight(height) => self.by_height.get(&height),
        }
        .cloned()
    }

    fn get_tip(&self) -> Option<BlockHeaderHash> {
        self.by_height
            .iter()
            .next_back()
            .map(|(_key, value)| value)
            .map(|block| block.as_ref().into())
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
