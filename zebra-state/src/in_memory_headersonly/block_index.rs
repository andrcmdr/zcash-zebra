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
    block::{BlockHeader, BlockHeaderHash},
    types::BlockHeight,
};

use super::QueryType;

type Error = Box<dyn error::Error + Send + Sync + 'static>;

#[derive(Default, Clone)]
pub(super) struct BlockIndex<T> {
    pub by_hash: HashMap<BlockHeaderHash, Arc<T>>,
    pub by_height: BTreeMap<BlockHeight, Arc<T>>,
    pub hash_height: HashMap<BlockHeaderHash, BlockHeight>,
}

#[derive(Copy, Clone)]
enum Either<Hash, Height, E> {
    Hash(Hash),
    Height(Height),
    HashHeight(Hash, Height),
    Error(E),
}

impl BlockIndex<BlockHeader> {
    pub fn insert(
        &mut self,
        block_header: impl Into<Arc<BlockHeader>>,
        block_height: BlockHeight,
    ) -> Result<(BlockHeaderHash, BlockHeight), Error> {
        let block_header = block_header.into();
        let hash: BlockHeaderHash = block_header.as_ref().into();
//      let height = block.coinbase_height().unwrap(); // didn't applicable for block height handling
        let height = block_height;

        let hash_result = match self.by_hash.entry(hash) {
            HashMapEntry::Vacant(entry) => {
                let _ = entry.insert(block_header.clone());
            //  let _ = self.by_hash.insert(hash, block_header);
                Either::<BlockHeaderHash, BlockHeight, _>::Hash(hash)
            }
            HashMapEntry::Occupied(_) => Either::Error(format!("Entry (block header) with this hash {:?} already exist", hash)),
        };

        let height_result = match self.by_height.entry(height) {
            BTreeMapEntry::Vacant(entry) => {
                let _ = entry.insert(block_header.clone());
            //  let _ = self.by_height.insert(height, block_header);
                Either::<BlockHeaderHash, BlockHeight, _>::Height(height)
            }
            BTreeMapEntry::Occupied(_) => Either::Error(format!("Entry (block header) with this height {:?} already exist", height)),
        };

        let hash_height_result = match self.hash_height.entry(hash) {
            HashMapEntry::Vacant(entry) => {
                let _ = entry.insert(height.clone());
            //  let _ = self.hash_height.insert(hash, height);
                Either::<BlockHeaderHash, BlockHeight, _>::HashHeight(hash, height)
            }
            HashMapEntry::Occupied(_) => Either::Error(format!("Entry (block header hash) with these hash {:?} & height {:?} already exist", hash, height)),
        };

        match (&hash_result, &height_result, &hash_height_result) {
            (Either::Hash(hash), Either::Height(height), Either::HashHeight(_header_hash, _header_height)) => Ok((hash.clone(), height.clone())),
            _ => {
                let mut error_result: String = String::from("");
//              [hash_result, height_result, hash_height_result]
//                  .iter()
//                  .map(|error| match error {
//                      Either::Error(e) => error_result = format!(" {:?} {:?};", error_result, e),
//                      _ => error_result = format!(" {:?};", error_result),
//                  } );
                [hash_result, height_result, hash_height_result]
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

    pub fn get(&self, query: impl Into<QueryType>) -> Result<Option<Arc<BlockHeader>>, Error> {
        let value = match query.into() {
            QueryType::ByHash(hash) => self.by_hash.get(&hash),
            QueryType::ByHeight(height) => self.by_height.get(&height),
        }
        .cloned();

        match value {
            Some(block_header) => Ok(Some(block_header)),
            None => Ok(None),
        }
    }

    pub fn get_tip(&self) -> Result<Option<(Arc<BlockHeader>, BlockHeaderHash, BlockHeight)>, Error> {
        let last_entry = self.by_height
            .iter()
            .next_back()
            .map(|(block_height, block_header)| (block_height.clone(), block_header.clone()));
//          .map(|block_header| block_header.as_ref().into()); // -> Option<BlockHeaderHash>

        match last_entry {
            Some((block_height, block_header)) => {
                let block_header_hash: BlockHeaderHash = block_header.as_ref().into();
                Ok(Some((block_header, block_header_hash, block_height)))
            },
            None => Ok(None),
        }
    }

    pub fn get_height(&self, hash: BlockHeaderHash) -> Result<Option<BlockHeight>, Error> {
        let height = self.hash_height.get(&hash);

        match height {
            Some(block_height) => Ok(Some(block_height.clone())),
            None => Ok(None),
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
