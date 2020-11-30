//! Application-local prelude: conveniently describes types/traits/functions/macros
//! which are generally/commonly useful and should be available everywhere.

#[derive(Clone, Debug)]
pub struct Config {
    pub cli_opts: String,
}

pub trait IBCRunnable {
    /// Run this `Runnable`
    fn run(&self, run_cli: Option<&str>);
}
