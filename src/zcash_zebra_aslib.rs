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

pub trait Runnable {
    /// Run this `Runnable`
    fn run(&self);
}

impl dyn Runnable {
    fn run(&self) {
//      let context = IBCItems{ hash: BlockHeaderHash([0u8; 32]), height: BlockHeight(0) };
//      IBCRunnable::run(&context);
        IBCRunnable::run(&IBCItems::default());
        async move {
            let _header_by_hash = IBCRequest::get(&IBCItems::default(), BlockHeaderHash([0u8; 32])).await;
            let _header_by_height = IBCRequest::get(&IBCItems::default(), BlockHeight(0)).await;
            let _tip = IBCRequest::get_tip(&IBCItems::default()).await;
        }.boxed();
    }
}
