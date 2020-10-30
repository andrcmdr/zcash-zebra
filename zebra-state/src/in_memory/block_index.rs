use std::{
    collections::{
        btree_map::Entry as BTreeMapEntry,
        hash_map::Entry as HashMapEntry,
        BTreeMap,
        HashMap
    },
    error,
    sync::Arc,
};

use zebra_chain::{
    block::{Block, BlockHeaderHash},
    types::BlockHeight,
};

use super::QueryType;

type Error = Box<dyn error::Error + Send + Sync + 'static>;

#[derive(Default, Clone)]
pub(super) struct BlockIndex<T> {
    pub by_hash: HashMap<BlockHeaderHash, Arc<T>>,
    pub by_height: BTreeMap<BlockHeight, Arc<T>>,
}

#[derive(Copy, Clone)]
enum Either<Hash, Height, E> {
    Hash(Hash),
    Height(Height),
    Error(E),
}

impl BlockIndex<Block> {
    pub fn insert(
        &mut self,
        block: impl Into<Arc<Block>>,
    ) -> Result<(BlockHeaderHash, BlockHeight), Error> {
        let block = block.into();
        let hash: BlockHeaderHash = block.as_ref().into();
        let height = block.coinbase_height().unwrap();

        let hash_result = match self.by_hash.entry(hash) {
            HashMapEntry::Vacant(entry) => {
                let _ = entry.insert(block.clone());
            //  let _ = self.by_hash.insert(hash, block);
                Either::<BlockHeaderHash, BlockHeight, _>::Hash(hash)
            }
            HashMapEntry::Occupied(_) => Either::Error(format!("Entry (block) with this hash {:?} already exist", hash))
        };

        let height_result = match self.by_height.entry(height) {
            BTreeMapEntry::Vacant(entry) => {
                let _ = entry.insert(block.clone());
            //  let _ = self.by_height.insert(height, block);
                Either::<BlockHeaderHash, BlockHeight, _>::Height(height)
            }
            BTreeMapEntry::Occupied(_) => Either::Error(format!("Entry (block) with this height {:?} already exist", height))
        };

        match (&hash_result, &height_result) {
            (Either::Hash(hash), Either::Height(height)) => Ok((hash.clone(), height.clone())),
            (Either::Error(_hash_error), Either::Error(_height_error)) => Err(format!("Entry (block) with this hash {:?} & height {:?} already exist", hash, height))?,
            _ => {
                let mut error_result: String = String::from("");
//              [hash_result, height_result]
//                  .iter()
//                  .map(|error| match error {
//                      Either::Error(e) => error_result = format!(" {:?} {:?};", error_result, e),
//                      _ => error_result = format!(" {:?};", error_result),
//                  } );
                [hash_result, height_result]
                    .iter()
                    .map(|error| if let Either::Error(e) = error {
                            error_result = format!(" {:?} {:?};", error_result, e)
                        } else {
                            error_result = format!(" {:?};", error_result)
                        }
                    );
                Err(format!("Error: {:?}", error_result))?
            },
        }
    }

    pub fn get(&self, query: impl Into<QueryType>) -> Result<Option<Arc<Block>>, Error> {
        let value = match query.into() {
            QueryType::ByHash(hash) => self.by_hash.get(&hash),
            QueryType::ByHeight(height) => self.by_height.get(&height),
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
            .map(|(_height, block)| block.clone());
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

/*
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
*/
