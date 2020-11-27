use zebrad::application::APPLICATION;
// use zebrad::prelude::*;
// use zebrad::prelude::Application as app;
// use zebrad::commands::ZebradCmd as cmd;
// use zebrad::commands::start_headersonly::StartHeadersOnlyCmd;
use std::path::{
//  Path,
    PathBuf,
};

use crate::prelude::*;

impl Default for Config {
    fn default() -> Self {
        Self {
            path: PathBuf::from("./zebrad.toml"),
        }
    }
}

impl IBCRunnable for Config {
    fn run(&self, config_file_path: Option<PathBuf>) {
        let filepath = match config_file_path {
            Some(fpath) => fpath.to_str().unwrap().to_owned(),
            None => Self::default().path.to_str().unwrap().to_owned(),
        };

    //  let arg = String::from("-c ./zebrad.toml start-headers-only");
    //  let arg = format!("-c {:?} start-headers-only", filepath);
    //  zebrad::prelude::Application::run(&APPLICATION, vec![arg].into_iter());
        zebrad::prelude::Application::run(&APPLICATION, vec!["-c".to_string(), filepath, "start-headers-only".to_string()].into_iter());
    //  zebrad::commands::ZebradCmd::StartHeadersOnly(StartHeadersOnlyCmd{ filters: vec!["".to_string()] }).run();
    //  zebrad::commands::ZebradCmd::StartHeadersOnly(StartHeadersOnlyCmd{ filters: Vec::new() }).run();
    }
}
