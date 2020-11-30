/*
use std::path::{
//  Path,
    PathBuf,
};
*/
use zebra_chain::{
    block::{
//      Block,
//      BlockHeader,
        BlockHeaderHash,
    },
    types::BlockHeight,
};
use zebra_state::{
//  QueryType,
//  RequestBlock,
    RequestBlockHeader,
    Response,
};
use std::{
    error,
//  error::Error,
//  future::Future,
};
use futures_util::FutureExt;
use tower::Service;

use ibcrun::prelude::*;
use ibclib::prelude::*;

type Error = Box<dyn error::Error + Send + Sync + 'static>;

const GENESIS: BlockHeaderHash = BlockHeaderHash([
    8, 206, 61, 151, 49, 176, 0, 192, 131, 56, 69, 92, 138, 74, 107, 208,
    93, 161, 110, 38, 177, 29, 170, 27, 145, 113, 132, 236, 232, 15, 4, 0,
]);

pub trait Runnable {
    /// Run this `Runnable`
    fn run(self);
}

impl<S> Runnable for IBCStorage<S>
where
    S: Service<RequestBlockHeader, Response = Response, Error = Error> + Send + Clone + 'static,
    S::Future: Send,
    Self: Send + Sync + 'static,
{
    fn run(self) {
        let conf_file_path = "./zebrad.toml";
        let state_dir_path = "./.zebra-state";
//      let arg_str = "-c ./zebrad.toml start-headers-only --cache-dir ./.zebra-state";
        let arg_str = format!("-c {:?} start-headers-only --cache-dir {:?}", conf_file_path, state_dir_path);
        let context = Config {  cli_opts: arg_str.clone() };

//      IBCRunnable::run(&context, None);
        IBCRunnable::run(&context, Some(arg_str.as_str()));
//      IBCRunnable::run(&Config::default(), None);
//      IBCRunnable::run(&Config::default(), Some(Config::default().cli_opts.as_str()));

        async move {
            let _header_by_height = IBCRequest::get(&self, BlockHeight(0)).await;
            let _header_by_hash = IBCRequest::get(&self, BlockHeaderHash([0u8; 32])).await;
            let _header_by_hash_ = IBCRequest::get(&self, GENESIS).await;
            let _tip = IBCRequest::get_tip(&self).await;
        }.boxed();
    }
}
