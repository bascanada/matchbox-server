use clap::Parser;
use matchbox_server::{args::Args, run, setup_logging};
use std::error::Error;
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    setup_logging();
    let args = Args::parse();
    let listener = TcpListener::bind(args.host).await?;
    run(listener.local_addr()?).await
}
