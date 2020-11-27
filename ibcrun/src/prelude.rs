//! Application-local prelude: conveniently describes types/traits/functions/macros
//! which are generally/commonly useful and should be available everywhere.

use std::path::{
//  Path,
    PathBuf,
};

#[derive(Clone, Debug)]
pub struct Config {
    pub path: PathBuf,
}

pub trait IBCRunnable {
    /// Run this `Runnable`
    fn run(&self, config_file_path: Option<PathBuf>);
}
