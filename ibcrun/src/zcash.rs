use zebrad::application::APPLICATION;
// use zebrad::prelude::*;
// use zebrad::prelude::Application as app;
// use zebrad::commands::ZebradCmd as cmd;
// use zebrad::commands::start_headersonly::StartHeadersOnlyCmd;
/*
use std::path::{
//  Path,
    PathBuf,
};
*/

use crate::prelude::*;

impl Default for Config {
    fn default() -> Self {
        Self {
            cli_opts: String::from("-c ./zebrad.toml start-headers-only --cache-dir ./.zebra-state"),
        }
    }
}

impl IBCRunnable for Config {
    fn run(&self, run_cli: Option<&str>) {
        let arg_str = match run_cli {
            Some(params) => params.to_owned(),
            None => Self::default().cli_opts,
        };

        let mut args = Vec::new();
        for s in arg_str.split(" ") {
            args.push(s.to_string())
        };

    //  zebrad::prelude::Application::run(&APPLICATION, vec!["-c".to_string(), conf_file_path, "start-headers-only".to_string()].into_iter());
    //  let arg_str = format!("-c {:?} start-headers-only", conf_file_path);
    //  let arg_str = String::from("-c ./zebrad.toml start-headers-only");
    //  zebrad::prelude::Application::run(&APPLICATION, vec![arg_str].into_iter());
        zebrad::prelude::Application::run(&APPLICATION, args.into_iter());
    /*  zebrad::commands::ZebradCmd::StartHeadersOnly(StartHeadersOnlyCmd{
            filters: Vec::new(),
        //  filters: vec!["".to_string()],
            cache_dir: PathBuf::from("./.zebra-state"),
            memory_cache_bytes: 1024 * 1024 * 1024,
            ephemeral: false,
        }).run();
    */
    }
}
