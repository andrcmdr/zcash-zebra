//! Zebrad Subcommands

pub mod connect;
pub mod connect_headersonly;
pub mod generate;
pub mod revhex;
pub mod seed;
pub mod start;
pub mod start_headersonly;
pub mod version;

use self::{
    connect::ConnectCmd,
    connect_headersonly::ConnectHeadersOnlyCmd,
    generate::GenerateCmd,
    revhex::RevhexCmd,
    seed::SeedCmd,
    start::StartCmd,
    start_headersonly::StartHeadersOnlyCmd,
    version::VersionCmd,
};
use crate::config::ZebradConfig;
use abscissa_core::{
    config::Override, Command, Configurable, FrameworkError, Help, Options, Runnable,
};
use std::path::PathBuf;

/// Zebrad Configuration Filename
pub const CONFIG_FILE: &str = "zebrad.toml";

/// Zebrad Subcommands
#[derive(Command, Debug, Options, Runnable)]
pub enum ZebradCmd {
    /// The `generate` subcommand
    #[options(help = "generate a skeleton configuration")]
    Generate(GenerateCmd),

    /// The `connect` subcommand
    #[options(help = "testing stub for dumping network messages about blocks requests")]
    Connect(ConnectCmd),

    /// The `connect-headers-only` subcommand
    #[options(help = "testing stub for dumping network messages about block headers requests")]
    ConnectHeadersOnly(ConnectHeadersOnlyCmd),

    /// The `help` subcommand
    #[options(help = "get usage information")]
    Help(Help<Self>),

    /// The `revhex` subcommand
    #[options(help = "reverses the endianness of a hex string, like a block or transaction hash")]
    Revhex(RevhexCmd),

    /// The `seed` subcommand
    #[options(help = "dns seeder")]
    Seed(SeedCmd),

    /// The `start` subcommand
    #[options(help = "start the application in blocks sync mode")]
    Start(StartCmd),

    /// The `start-headers-only` subcommand
    #[options(help = "start the application in block headers sync mode")]
    StartHeadersOnly(StartHeadersOnlyCmd),

    /// The `version` subcommand
    #[options(help = "display version information")]
    Version(VersionCmd),
}

/// This trait allows you to define how application configuration is loaded.
impl Configurable<ZebradConfig> for ZebradCmd {
    /// Location of the configuration file
    fn config_path(&self) -> Option<PathBuf> {
        let filename = std::env::current_dir().ok().map(|mut dir_path| {
            dir_path.push(CONFIG_FILE);
            dir_path
        });

        let if_exists = |f: PathBuf| if f.exists() { Some(f) } else { None };

        filename.and_then(if_exists)

        // Note: Changes in how configuration is loaded may need usage
        // edits in generate.rs
    }

    /// Apply changes to the config after it's been loaded, e.g. overriding
    /// values in a config file using command-line options.
    ///
    /// This can be safely deleted if you don't want to override config
    /// settings from command-line options.
    fn process_config(&self, config: ZebradConfig) -> Result<ZebradConfig, FrameworkError> {
        match self {
            ZebradCmd::Start(cmd) => cmd.override_config(config),
            ZebradCmd::StartHeadersOnly(cmd) => cmd.override_config(config),
            _ => Ok(config),
        }
    }
}
