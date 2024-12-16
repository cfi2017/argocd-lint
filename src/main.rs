use anyhow::Context;
use std::hash::Hash;
use std::path::PathBuf;
use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[clap(short, long, default_value = "config", value_name = "FILE")]
    config: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    
    let config = argocd_lint::config::Config::load(args.config).context("could not load config")?;

    argocd_lint::check(config).await
}
