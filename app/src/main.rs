use std::env::set_var;

use cli::error::CliResult;
use tracing_subscriber::EnvFilter;

mod config;
mod server;
mod cli;

#[tokio::main]
async fn main() -> CliResult<()> {
    set_var("RUST_LOG", "none,ocloud=trace");

    tracing_subscriber::fmt()
        .with_target(true)
        .with_thread_ids(true)
        .with_line_number(true)
        .with_env_filter(EnvFilter::from_default_env())
        .init();

    cli::run().await
}
