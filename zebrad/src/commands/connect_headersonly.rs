//! `connect-headers-only` subcommand - test stub for talking to zcashd

use crate::{components::tokio::TokioComponent, prelude::*};
use abscissa_core::{Command, Options, Runnable};
use color_eyre::eyre::{eyre, Report, WrapErr};
use futures::{
//  prelude::*,
    stream::{FuturesUnordered, StreamExt},
};
use std::sync::Arc;
use std::collections::BTreeSet;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use tower::{buffer::Buffer, service_fn, Service, ServiceExt};

use zebra_chain::{
    block::{
//      Block,
        BlockHeader,
        BlockHeaderHash
    },
    types::BlockHeight,
};

// use zebra_state::QueryType;

// genesis
static GENESIS: BlockHeaderHash = BlockHeaderHash([
    8, 206, 61, 151, 49, 176, 0, 192, 131, 56, 69, 92, 138, 74, 107, 208, 93, 161, 110, 38, 177,
    29, 170, 27, 145, 113, 132, 236, 232, 15, 4, 0,
]);

/// `connect-headers-only` subcommand
#[derive(Command, Debug, Options)]
pub struct ConnectHeadersOnlyCmd {
    /// The address of the node to connect to.
    #[options(
        help = "The address of the node to connect to.",
        default = "127.0.0.1:8233"
    )]
    addr: SocketAddr,
}

impl Default for ConnectHeadersOnlyCmd {
    fn default() -> Self {
        Self {
            addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8233),
        }
    }
}

impl Runnable for ConnectHeadersOnlyCmd {
    /// Start the application.
    fn run(&self) {
        info!(connect.addr = ?self.addr);

        let rt = app_writer()
            .state_mut()
            .components
            .get_downcast_mut::<TokioComponent>()
            .expect("TokioComponent should be available")
            .rt
            .take();

        let result = rt
            .expect("runtime should not already be taken")
            .block_on(self.connect());

        match result {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error: {:?}", e);
                std::process::exit(1);
            }
        }
    }
}

impl ConnectHeadersOnlyCmd {
    async fn connect(&self) -> Result<(), Report> {
        info!(?self, "begin tower-based peer handling test stub");

        // The service that our node uses to respond to requests by peers
        let node = Buffer::new(
            service_fn(|req| async move {
                info!(?req);
                Ok::<zebra_network::Response, Report>(zebra_network::Response::Nil)
            }),
            1,
        );

        let mut config = app_config().network.clone();
        // Use a different listen addr so that we don't conflict with another local node.
        config.listen_addr = "0.0.0.0:38233".parse()?;
        // Connect only to the specified peer.
        config.initial_mainnet_peers.insert(self.addr.to_string());

        let state = zebra_state::in_memory_headersonly::init();
        let (peer_set, _address_book) = zebra_network::init(config, node).await;
        let retry_peer_set = tower::retry::Retry::new(zebra_network::RetryErrors, peer_set.clone());

        let mut downloaded_block_heights = BTreeSet::<BlockHeight>::new();
        downloaded_block_heights.insert(BlockHeight(0));

        let mut connect = ConnectHeadersOnly {
            retry_peer_set,
            peer_set,
            state,
            tip: GENESIS,
            block_requests: FuturesUnordered::new(),
            requested_block_heights: 0,
            downloaded_block_heights,
        };

        connect.connect().await
    }
}

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

struct ConnectHeadersOnly<ZN, ZS>
where
    ZN: Service<zebra_network::Request>,
{
    retry_peer_set: tower::retry::Retry<zebra_network::RetryErrors, ZN>,
    peer_set: ZN,
    state: ZS,
    tip: BlockHeaderHash,
    block_requests: FuturesUnordered<ZN::Future>,
    requested_block_heights: usize,
    downloaded_block_heights: BTreeSet<BlockHeight>,
}

impl<ZN, ZS> ConnectHeadersOnly<ZN, ZS>
where
    ZN: Service<zebra_network::Request, Response = zebra_network::Response, Error = Error>
        + Send
        + Clone
        + 'static,
    ZN::Future: Send,
    ZS: Service<zebra_state::RequestBlockHeader, Response = zebra_state::Response, Error = Error>
        + Send
        + Clone
        + 'static,
    ZS::Future: Send,
{
    async fn connect(&mut self) -> Result<(), Report> {
        // TODO(jlusby): Replace with real state service

//      while self.requested_block_heights < 1_000_000 {
        loop {
            let hashes = self.next_hashes().await?;
            self.tip = *hashes.last().unwrap();

            // Request the corresponding blocks in chunks
            self.request_blocks(hashes).await?;

            // Allow at most 500 block requests in flight.
            self.drain_requests(500).await?;
        }

//      self.drain_requests(0).await?;

//      let eternity = future::pending::<()>();
//      eternity.await;

//      Ok(())
    }

    async fn next_hashes(&mut self) -> Result<Vec<BlockHeaderHash>, Report> {
        // Request the next 500 hashes.
        self.retry_peer_set
            .ready_and()
            .await
            .map_err(|e| eyre!(e))?
            .call(zebra_network::Request::FindBlocks {
                known_blocks: vec![self.tip],
                stop: None,
            })
            .await
            .map_err(|e| eyre!(e))
            .wrap_err("request failed, TODO implement retry")
            .map(|response| match response {
                zebra_network::Response::BlockHeaderHashes(hashes) => hashes,
                _ => unreachable!("FindBlocks always gets a BlockHeaderHashes response"),
            })
            .map(|hashes| {
             /* let mut latest_block_header_hash = String::from("");
                for byte in self.downloaded_block_header_hashes.iter().last().unwrap().0.iter() {
                    latest_block_header_hash.push_str(format!("{:02x}", byte).as_str())
                };
                let latest_block_header_hash = hex::encode(self.downloaded_block_header_hashes.iter().last().unwrap().0); */
                info!(
                    new_hashes = hashes.len(),
                    requested = self.requested_block_heights,
                    in_flight = self.block_requests.len(),
                    downloaded = self.downloaded_block_heights.len(),
                    highest = self.downloaded_block_heights.iter().next_back().unwrap().0,
                    "requested more hashes"
                );
                self.requested_block_heights += hashes.len();
                hashes
            })
    }

    async fn request_blocks(&mut self, hashes: Vec<BlockHeaderHash>) -> Result<(), Report> {
        for chunk in hashes.chunks(10usize) {
            let request = self.peer_set.ready_and().await.map_err(|e| eyre!(e))?.call(
                zebra_network::Request::BlocksByHash(chunk.iter().cloned().collect()),
            );

            self.block_requests.push(request);
        }

        Ok(())
    }

    async fn drain_requests(&mut self, request_goal: usize) -> Result<(), Report> {
        while self.block_requests.len() > request_goal {
            match self.block_requests.next().await {
                Some(Ok(zebra_network::Response::Blocks(blocks))) => {
                    for block in blocks {
                        let header: Arc<BlockHeader> = block.header.into();
                        let hash: BlockHeaderHash = block.as_ref().into();
                        let _hash_str = hex::encode(&hash.0);
                        let height = block.coinbase_height().unwrap();

                        self.downloaded_block_heights
                            .insert(height);

                        self.state
                            .ready_and()
                            .await
                            .map_err(|e| eyre!(e))?
                            .call(zebra_state::RequestBlockHeader::AddBlockHeader { block_header: header, block_height: height })
                            .await
                            .map_err(|e| eyre!(e))?;
                    }
                }
                Some(Err(e)) => {
                    error!(%e);
                }
                _ => continue,
            }
        }
        Ok(())
    }
}
