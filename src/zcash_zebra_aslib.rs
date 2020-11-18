use std::path::{
//  Path,
    PathBuf,
};

use zebra_chain::{
    block::{
//      Block,
//      BlockHeader,
        BlockHeaderHash,
    },
    types::BlockHeight,
};

use std::{
    error,
//  error::Error,
//  future::Future,
};

use futures_util::FutureExt;

use ibclib::prelude::*;

type Error = Box<dyn error::Error + Send + Sync + 'static>;

const GENESIS: BlockHeaderHash = BlockHeaderHash([
    8, 206, 61, 151, 49, 176, 0, 192, 131, 56, 69, 92, 138, 74, 107, 208,
    93, 161, 110, 38, 177, 29, 170, 27, 145, 113, 132, 236, 232, 15, 4, 0,
]);

pub trait Runnable {
    /// Run this `Runnable`
    fn run(&self);
}

impl dyn Runnable {
    fn run(&self) {
//      let context = Config { path: PathBuf::from("./zebrad.toml") };
        let filepath = PathBuf::from("./zebrad.toml");
//      IBCRunnable::run(&context, None);
//      IBCRunnable::run(&context, Some(filepath));
//      IBCRunnable::run(&Config::default(), None);
        IBCRunnable::run(&Config::default(), Some(filepath));

        async move {
            let _header_by_hash = IBCRequest::get(&IBCItems::default(), BlockHeaderHash([0u8; 32])).await;
            let _header_by_height = IBCRequest::get(&IBCItems::default(), BlockHeight(0)).await;
            let _tip = IBCRequest::get_tip(&IBCItems::default()).await;

//          let context = IBCItems{ hash: BlockHeaderHash([0u8; 32]), height: BlockHeight(0) };
            let context = IBCItems{ hash: GENESIS, height: BlockHeight(0) };
            let _header_by_hash_ = IBCRequest::get(&context, GENESIS).await;
            let _header_by_height_ = IBCRequest::get(&context, BlockHeight(0)).await;
            let _tip_ = IBCRequest::get_tip(&context).await;
        }.boxed();
    }
}
