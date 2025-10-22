use clap::Parser;
use matchbox_server::{args::Args, run, setup_logging};
use std::error::Error;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    setup_logging();
    let args = Args::parse();
    run(args.host).await
}
