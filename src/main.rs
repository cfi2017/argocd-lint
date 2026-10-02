use anyhow::Context;
use std::path::PathBuf;
use clap::Parser;

#[derive(Parser)]
#[command(version, about, long_about = None)]
struct Cli {
    #[clap(short, long, default_value = "config", value_name = "FILE")]
    config: Option<PathBuf>,
    #[clap(long)]
    output_namespaced_resource_whitelist: bool,
    #[clap(long, value_name = "NAME")]
    cluster: Option<String>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Cli::parse();
    
    let config = argocd_lint::config::Config::load(args.config).context("could not load config")?;
    let clusters = config.selected_clusters(args.cluster.as_deref())?;
    let mut succeeded = true;

    for (name, cluster) in clusters {
        eprintln!("validating cluster {}", name);
        succeeded &= argocd_lint::check(
            &config,
            &cluster.entrypoints,
            cluster.check_namespaces,
            args.output_namespaced_resource_whitelist,
        )
        .await?;
    }

    if !succeeded {
        std::process::exit(1);
    }

    Ok(())
}
