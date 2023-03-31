use clap::Parser;

mod args;
use args::{Args, Command};

mod reset_dynamodb;

mod util;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    match args.command {
        Command::ResetDynamodb => reset_dynamodb::run().await,
    }
}
