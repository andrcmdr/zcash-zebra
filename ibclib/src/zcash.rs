use zebrad::application::APPLICATION;
// use zebrad::prelude::*;
// use zebrad::prelude::Application as app;
// use zebrad::commands::ZebradCmd as cmd;
// use zebrad::commands::start_headersonly::StartHeadersOnlyCmd;
use zebra_chain::{
    block::{
        Block,
        BlockHeader,
        BlockHeaderHash,
    },
    types::BlockHeight,
};
use zebra_state::{
    QueryType,
//  RequestBlock,
    RequestBlockHeader,
    Response,
};
use std::{
    error,
    future::Future,
    pin::Pin,
    sync::Arc,
};
use futures_util::FutureExt;
use tower::{Service, ServiceExt};

type Error = Box<dyn error::Error + Send + Sync + 'static>;

use crate::prelude::*;

impl dyn IBCRunnable {
    pub fn run(&self) {
        //  let arg = String::from("-c ./zebrad.toml start-headers-only");
        //  zebrad::prelude::Application::run(&APPLICATION, vec![arg].into_iter());
            zebrad::prelude::Application::run(&APPLICATION, vec!["-c".to_string(), "./zebrad.toml".to_string(), "start-headers-only".to_string()].into_iter());
        //  zebrad::commands::ZebradCmd::StartHeadersOnly(StartHeadersOnlyCmd{ filters: vec!["".to_string()] }).run();
        //  zebrad::commands::ZebradCmd::StartHeadersOnly(StartHeadersOnlyCmd{ filters: Vec::new() }).run();
        }
}

impl From<BlockHeaderHash> for IBCQuery<BlockHeaderHash, BlockHeight> {
    fn from(hash: BlockHeaderHash) -> Self {
        Self::ByHash(hash)
    }
}

impl From<BlockHeight> for IBCQuery<BlockHeaderHash, BlockHeight> {
    fn from(height: BlockHeight) -> Self {
        Self::ByHeight(height)
    }
}

impl IBCRequest<BlockHeaderHash, BlockHeight> for IBCItems<BlockHeaderHash, BlockHeight> {
    type BlockResponse = Pin<Box<dyn Future<Output = Result<Option<Arc<Block>>, Error>> + Send + 'static>>;
    type HeaderResponse = Pin<Box<dyn Future<Output = Result<Option<Arc<BlockHeader>>, Error>> + Send + 'static>>;
    type HashResponse = Pin<Box<dyn Future<Output = Result<Option<BlockHeaderHash>, Error>> + Send + 'static>>;
    type HeightResponse = Pin<Box<dyn Future<Output = Result<Option<BlockHeight>, Error>> + Send + 'static>>;
    type HashHeightResponse = Pin<Box<dyn Future<Output = Result<Option<(BlockHeaderHash, BlockHeight)>, Error>> + Send + 'static>>;

    fn get(&self, query: impl Into<IBCQuery<BlockHeaderHash, BlockHeight>>) -> Self::HeaderResponse {
        let state = zebra_state::on_disk_headersonly::init(zebra_state::Config::default());
        let value = match query.into() {
            IBCQuery::ByHash(hash) => {
                let mut state = state.clone();
                async move {
                    let get_block_header = state
                    .ready_and()
                    .await?
                    .call(RequestBlockHeader::GetBlockHeader { query: QueryType::ByHash(hash) });

                    tracing::info!("Block header with hash {:?} requested!", hash);

                    match get_block_header.await? {
                        zebra_state::Response::BlockHeader { block_header } => Ok(Some(block_header)),
                        _ => Err("block header couldn't be found - either still syncing, or out of range".into()),
                    }
                }.boxed()
            }
            IBCQuery::ByHeight(height) => {
                let mut state = state.clone();
                async move {
                    let get_block_header = state
                    .ready_and()
                    .await?
                    .call(RequestBlockHeader::GetBlockHeader { query: QueryType::ByHeight(height) });

                    tracing::info!("Block header with height {:?} requested!", height);

                    match get_block_header.await? {
                        zebra_state::Response::BlockHeader { block_header } => Ok(Some(block_header)),
                        _ => Err("block header couldn't be found - either still syncing, or out of range".into()),
                    }
                }.boxed()
            }
        };
        value
    }

    fn get_tip(&self) -> Self::HashHeightResponse {
        let state = zebra_state::on_disk_headersonly::init(zebra_state::Config::default());
        let mut state = state.clone();
        async move {
            let get_tip = state
            .ready_and()
            .await?
            .call(RequestBlockHeader::GetTip);

            tracing::info!("Tip requested!");

            match get_tip.await? {
                Response::Tip { hash, height } => Ok(Some((hash, height))),
                _ => Err("Some error in requesting block header that is the tip of the current chain".into()),
            }
        }.boxed()
    }
}
