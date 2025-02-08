use std::sync::Arc;

use api::method::rpc_method_handler;
use api::status::rpc_status_handler;
use args::CliArgs;
use clap::Parser;
use crawler::Crawler;
use jsonlogger::JsonLogger;
use jsonrpsee::server::Server;
use jsonrpsee::RpcModule;
use simplelog::{ConfigBuilder, TermLogger};

mod api;
mod args;
mod crawler;
mod jsonlogger;
mod yamlparser;

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();

    if args.json_logs {
        JsonLogger::init(args.verbosity.into(), std::io::stderr()).unwrap();
    } else {
        TermLogger::init(
            args.verbosity.into(),
            ConfigBuilder::new().build(),
            simplelog::TerminalMode::Stderr,
            simplelog::ColorChoice::Auto,
        )
        .unwrap();
    }

    log::info!("Watching folder: {}", args.folder.to_string_lossy());
    log::info!(
        "Starting server on address and port: {}:{}",
        args.addr,
        args.port
    );

    let full_addr = format!("{}:{}", args.addr, args.port);

    let server = Server::builder().build(full_addr).await.unwrap();
    let actual_addr = server.local_addr().expect("Could not get server address");

    // Print the port to stdout
    println!("{}", actual_addr.port());

    let crawler = Arc::new(Crawler::new(args.folder).await);
    crawler.start().await;

    let mut module = RpcModule::from_arc(crawler);
    module
        .register_async_method("status", rpc_status_handler)
        .unwrap();

    module
        .register_async_method("method", rpc_method_handler)
        .unwrap();

    let handle = server.start(module);

    log::info!("Started server");

    tokio::spawn(handle.stopped()).await.unwrap();
}
