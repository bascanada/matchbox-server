//! A command-line tool to demonstrate the client-side authentication flow.
use anyhow::Result;
use clap::Parser;
use matchbox_server::helpers;
use serde_json::Value;

#[derive(Parser, Debug)]
#[clap(name = "client-auth-demo")]
struct Args {
    #[clap(short, long)]
    username: String,
    #[clap(short, long)]
    password: String,
    #[clap(short, long)]
    challenge: String,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    let payload = helpers::generate_login_payload(&args.username, &args.password, &args.challenge)?;
    let value: Value = serde_json::from_str(&payload)?;
    println!("{}", serde_json::to_string_pretty(&value)?);
    Ok(())
}
