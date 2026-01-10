mod admin;
mod backend;
mod balancer;
mod bench;
mod cache;
mod config;
mod error;
mod metrics;
mod middleware;
mod proxy;
mod routing;
mod tls;
mod utils;

use crate::backend::{HealthChecker, Server, ServerPool};
use crate::config::HealthCheckConfig;
use crate::balancer::{Algorithm, LoadBalancerSelector};
use crate::config::Config;
use crate::metrics::MetricsCollector;
use crate::proxy::ProxyServer;
use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use std::sync::Arc;
use tracing::{error, info};
use tracing_subscriber::{fmt, prelude::*, EnvFilter};

#[derive(Parser)]
#[command(name = "ultrabalancer")]
#[command(author = "Kira <kiraa@tuta.io>")]
#[command(version = "1.0.0")]
#[command(about = "Production-ready high-performance load balancer", long_about = None)]
#[command(styles = get_styles())]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    #[arg(short = 'p', long, default_value = "8080")]
    port: u16,

    #[arg(long, default_value = "0.0.0.0")]
    host: String,

    #[arg(short = 'a', long, default_value = "round-robin")]
    algorithm: String,

    #[arg(short = 'b', long = "backend", value_name = "HOST:PORT")]
    backends: Vec<String>,

    #[arg(short = 'c', long)]
    config: Option<String>,

    #[arg(long)]
    no_health_check: bool,

    #[arg(long, default_value = "5000")]
    health_check_interval: u64,

    #[arg(long, default_value = "3")]
    health_check_fails: u32,

    #[arg(short = 'w', long, default_value = "100")]
    weight: u32,
}

#[derive(Subcommand)]
enum Commands {
    Start {
        algorithm: String,
        backends: Vec<String>,
        #[arg(short = 'p', long, default_value = "8080")]
        port: u16,
        #[arg(long, default_value = "100")]
        weight: u32,
        #[arg(long)]
        no_health_check: bool,
    },

    Validate {
        #[arg(short = 'f', long)]
        config: String,
    },

    Example,

    Info,
}

fn get_styles() -> clap::builder::Styles {
    clap::builder::Styles::styled()
        .usage(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
        )
        .header(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Yellow))),
        )
        .literal(
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
        )
        .invalid(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))),
        )
        .error(
            anstyle::Style::new()
                .bold()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Red))),
        )
        .valid(
            anstyle::Style::new()
                .bold()
                .underline()
                .fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::Green))),
        )
        .placeholder(
            anstyle::Style::new().fg_color(Some(anstyle::Color::Ansi(anstyle::AnsiColor::White))),
        )
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::registry()
        .with(fmt::layer().with_target(false).compact())
        .with(
            EnvFilter::from_default_env()
                .add_directive(tracing::Level::INFO.into())
        )
        .init();

    let cli = Cli::parse();

    match cli.command {
        Some(Commands::Start {
            algorithm,
            backends,
            port,
            weight,
            no_health_check,
        }) => {
            execute_start(algorithm, backends, port, weight, !no_health_check).await?;
        }
        Some(Commands::Validate { config }) => {
            execute_validate(&config)?;
        }
        Some(Commands::Example) => {
            execute_example();
        }
        Some(Commands::Info) => {
            execute_info();
        }
        None => {
            if let Some(config_path) = cli.config {
                execute_with_config_file(&config_path).await?;
            } else if !cli.backends.is_empty() {
                execute_with_cli_args(cli).await?;
            } else {
                error!("No configuration provided");
                print_usage_examples();
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

async fn execute_start(
    algorithm: String,
    backends: Vec<String>,
    port: u16,
    weight: u32,
    health_enabled: bool,
) -> Result<()> {
    print_banner();

    let algo = Algorithm::from_str(&algorithm)
        .context(format!("Invalid algorithm: {}", algorithm))?;

    let server_list: Result<Vec<Server>> = backends
        .iter()
        .map(|b| parse_backend(b, weight))
        .collect();

    info!("⚙️  Configuration:");
    info!("   Algorithm: {}", algo.as_str());
    info!("   Listen: 0.0.0.0:{}", port);
    info!("   Health Checks: {}", if health_enabled { "✓ enabled" } else { "✗ disabled" });

    execute_load_balancer(
        format!("0.0.0.0:{}", port),
        algo,
        server_list?,
        health_enabled,
        5000,
        3,
    )
    .await
}

async fn execute_with_cli_args(cli: Cli) -> Result<()> {
    print_banner();

    let algo = Algorithm::from_str(&cli.algorithm)
        .context(format!("Invalid algorithm: {}", cli.algorithm))?;

    let server_list: Result<Vec<Server>> = cli
        .backends
        .iter()
        .map(|b| parse_backend(b, cli.weight))
        .collect();

    let health_enabled = !cli.no_health_check;

    info!("⚙️  Configuration:");
    info!("   Algorithm: {}", algo.as_str());
    info!("   Listen: {}:{}", cli.host, cli.port);
    info!("   Health Checks: {}", if health_enabled { "✓ enabled" } else { "✗ disabled" });

    execute_load_balancer(
        format!("{}:{}", cli.host, cli.port),
        algo,
        server_list?,
        health_enabled,
        cli.health_check_interval,
        cli.health_check_fails,
    )
    .await
}

async fn execute_with_config_file(path: &str) -> Result<()> {
    print_banner();
    info!("📄 Loading configuration from {}", path);

    let config = Config::from_file(path)?;
    config.validate()?;

    let algo = Algorithm::from_str(&config.algorithm)
        .context(format!("Invalid algorithm: {}", config.algorithm))?;

    let server_list: Vec<Server> = config
        .backends
        .iter()
        .map(|b| Server::new(b.host.clone(), b.port, b.weight))
        .collect();

    info!("⚙️  Configuration:");
    info!("   Algorithm: {}", algo.as_str());
    info!("   Listen: {}:{}", config.listen_address, config.listen_port);

    execute_load_balancer(
        format!("{}:{}", config.listen_address, config.listen_port),
        algo,
        server_list,
        config.health_check.enabled,
        config.health_check.interval_ms,
        config.health_check.max_failures,
    )
    .await
}

async fn execute_load_balancer(
    listen_addr: String,
    algorithm: Algorithm,
    servers: Vec<Server>,
    health_enabled: bool,
    health_interval_ms: u64,
    max_failures: u32,
) -> Result<()> {
    if servers.is_empty() {
        anyhow::bail!("No backend servers configured");
    }

    info!("🎯 Backends ({}):", servers.len());
    for server in &servers {
        info!("   → {} [weight: {}]", server.address(), server.weight);
    }

    let pool = ServerPool::new(servers);
    let selector = LoadBalancerSelector::new(algorithm);
    let metrics = Arc::new(MetricsCollector::new());

    let health_config = HealthCheckConfig {
        enabled: health_enabled,
        interval_ms: health_interval_ms,
        max_failures,
        path: "/".to_string(),
        expected_status: 200,
        timeout_ms: 2000,
        headers: std::collections::HashMap::new(),
        expected_body: None,
        circuit_breaker: None,
    };

    let health_checker = Arc::new(HealthChecker::new(pool.clone(), health_config));

    let proxy = Arc::new(ProxyServer::new(
        selector,
        pool,
        metrics,
        listen_addr,
    ));

    tokio::spawn(async move {
        health_checker.start().await;
    });

    info!("🚀 Load balancer starting...");
    proxy.start().await?;

    Ok(())
}

fn parse_backend(backend: &str, default_weight: u32) -> Result<Server> {
    let parts: Vec<&str> = backend.split(':').collect();
    if parts.len() != 2 {
        anyhow::bail!("Invalid backend format: {}. Expected host:port", backend);
    }

    let host = parts[0].to_string();
    let port: u16 = parts[1]
        .parse()
        .context(format!("Invalid port in backend: {}", backend))?;

    Ok(Server::new(host, port, default_weight))
}

fn execute_validate(path: &str) -> Result<()> {
    let config = Config::from_file(path)?;
    config.validate()?;
    info!("✓ Configuration is valid");
    println!("\nSummary:");
    println!("  Listen: {}:{}", config.listen_address, config.listen_port);
    println!("  Algorithm: {}", config.algorithm);
    println!("  Backends: {}", config.backends.len());
    Ok(())
}

fn execute_example() {
    let example = r#"# UltraBalancer Configuration

listen_address: "0.0.0.0"
listen_port: 8080
algorithm: "round-robin"

backends:
  - host: "192.168.1.10"
    port: 8080
    weight: 100
  - host: "192.168.1.11"
    port: 8080
    weight: 100
  - host: "192.168.1.12"
    port: 8080
    weight: 50

health_check:
  enabled: true
  interval_ms: 5000
  max_failures: 3

timeout:
  connect_ms: 5000
  request_ms: 30000
"#;

    println!("{}", example);
}

fn execute_info() {
    println!("UltraBalancer v1.0.0");
    println!("Production-grade load balancer written in Rust\n");
    println!("Supported Algorithms:");
    println!("  • round-robin       - Distribute requests evenly");
    println!("  • least-connections - Route to server with fewest connections");
    println!("  • ip-hash          - Consistent hashing based on client IP");
    println!("  • random           - Random distribution");
    println!("  • weighted         - Weight-based round robin\n");
    println!("Features:");
    println!("  • Automatic health checking");
    println!("  • Real-time metrics (/metrics, /prometheus endpoints)");
    println!("  • Zero-downtime failover");
    println!("  • HTTP/1.1 proxying");
    println!("  • Connection pooling");
    println!("  • Circuit breaker pattern");
}

fn print_banner() {
    println!("\n╔══════════════════════════════════════════╗");
    println!("║      UltraBalancer v2.0.0                ║");
    println!("║  Production Load Balancer                ║");
    println!("╚══════════════════════════════════════════╝\n");
}

fn print_usage_examples() {
    eprintln!("\n Usage examples:");
    eprintln!("  ultrabalancer start round-robin server1:8080 server2:8080 -p 80");
    eprintln!("  ultrabalancer -c config.yaml");
    eprintln!("  ultrabalancer -b 10.0.0.1:8080 -b 10.0.0.2:8080 -a least-connections");
}
