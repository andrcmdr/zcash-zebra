//! `start` subcommand - entry point for starting a zebra node
//!
//!  ## Application Structure
//!
//!  A zebra node consists of the following services and tasks:
//!
//!  * Network Service
//!    * primary interface to the node
//!    * handles all external network requests for the Zcash protocol
//!      * via zebra_network::Message and zebra_network::Response
//!    * provides an interface to the rest of the network for other services and
//!    tasks running within this node
//!      * via zebra_network::Request
//!  * Consensus Service
//!    * handles all validation logic for the node
//!    * verifies blocks using zebra-chain and zebra-script, then stores verified
//!    blocks in zebra-state
//!  * Sync Task
//!    * This task runs in the background and continuously queries the network for
//!    new blocks to be verified and added to the local state
use crate::config::ZebradConfig;
use crate::{components::tokio::TokioComponent, prelude::*};
use abscissa_core::{config, Command, FrameworkError, Options, Runnable};
use color_eyre::eyre::Report;
use tower::{buffer::Buffer, service_fn};
use zebra_chain::{
    block::BlockHeaderHash,
//  block::{Block, BlockHeader, BlockHeaderHash},
};
use std::path::{
//  Path,
    PathBuf,
};

mod sync;

// genesis
const GENESIS: BlockHeaderHash = BlockHeaderHash([
    8, 206, 61, 151, 49, 176, 0, 192, 131, 56, 69, 92, 138, 74, 107, 208, 93, 161, 110, 38, 177,
    29, 170, 27, 145, 113, 132, 236, 232, 15, 4, 0,
]);

/// `start` subcommand
#[derive(Command, Debug, Options)]
pub struct StartCmd {
    /// The filter strings used for tracing events.
    // #[options(free, help = "The filter strings used for tracing events")]
    #[options(help = "The filter strings used for tracing events")]
    pub filters: Vec<String>,
    /// Configuration for the state service.
    /// The root directory for storing cached data into the state storage.
    #[options(help = "The root directory for storing cached data into the state storage")]
    pub cache_dir: Option<PathBuf>,
    /// The maximum number of bytes to use caching data in memory.
    #[options(help = "The maximum number of bytes to use caching data in memory")]
    pub memory_cache_bytes: Option<u64>,
    /// Whether to use an ephemeral database.
    /// Ephemeral databases are stored in memory on Linux, and in a temporary directory on other OSes.
    /// Set to `false` by default. If this is set to `true`, [`cache_dir`] is ignored.
    #[options(help = "Whether to use an ephemeral database (stored in memory)")]
    pub ephemeral: bool,
}

impl StartCmd {
    async fn start(&self) -> Result<(), Report> {
        info!(?self, "begin tower-based peer handling test stub");

        // The service that our node uses to respond to requests by peers
        let node = Buffer::new(
            service_fn(|req| async move {
                info!(?req);
                Ok::<zebra_network::Response, Report>(zebra_network::Response::Nil)
            }),
            1,
        );
        let config = app_config();
        let state = zebra_state::on_disk::init(config.state.clone());
        let (peer_set, _address_book) = zebra_network::init(config.network.clone(), node).await;
        let verifier = zebra_consensus::verify::block::init(state.clone());

        let mut syncer = sync::Syncer::new(peer_set, state, verifier);

        syncer.sync().await
    }
}

impl Runnable for StartCmd {
    /// Start the application.
    fn run(&self) {
        let rt = app_writer()
            .state_mut()
            .components
            .get_downcast_mut::<TokioComponent>()
            .expect("TokioComponent should be available")
            .rt
            .take();

        let result = rt
            .expect("runtime should not already be taken")
            .block_on(self.start());

        match result {
            Ok(()) => {}
            Err(e) => {
                eprintln!("Error: {:?}", e);
                std::process::exit(1);
            }
        }
    }
}

impl config::Override<ZebradConfig> for StartCmd {
    // Process the given command line options, overriding settings from
    // a configuration file using explicit flags taken from command-line
    // arguments.
    fn override_config(&self, mut config: ZebradConfig) -> Result<ZebradConfig, FrameworkError> {
        if !self.filters.is_empty() {
            config.tracing.filter = Some(self.filters.join(","));
        }
        if let Some(dir) = self.cache_dir.to_owned() {
            config.state.cache_dir = dir;
        }
        if let Some(mem_size) = self.memory_cache_bytes {
            config.state.memory_cache_bytes = mem_size;
        }
        if self.ephemeral {
            config.state.ephemeral = self.ephemeral;
        }

        Ok(config)
    }
}
