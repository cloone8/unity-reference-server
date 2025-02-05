use std::collections::HashMap;
use std::sync::Arc;

use args::CliArgs;
use clap::Parser;
use crawler::{Crawler, Method, MethodRef, ServerStatus};
use jsonlogger::JsonLogger;
use jsonrpsee::server::Server;
use jsonrpsee::types::Params;
use jsonrpsee::{Extensions, ResponsePayload, RpcModule};
use simplelog::{ConfigBuilder, TermLogger};

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
        .register_async_method("all", rpc_all_refs_handler)
        .unwrap();

    module
        .register_async_method("method", rpc_method_handler)
        .unwrap();

    let handle = server.start(module);

    log::info!("Started server");

    tokio::spawn(handle.stopped()).await.unwrap();
}

async fn rpc_status_handler(
    _params: Params<'static>,
    context: Arc<Crawler>,
    _extensions: Extensions,
) -> ResponsePayload<'static, ServerStatus> {
    log::debug!("Handling status request");

    ResponsePayload::success(context.status.read().await.clone())
}

async fn rpc_all_refs_handler(
    _params: Params<'static>,
    context: Arc<Crawler>,
    _extensions: Extensions,
) -> ResponsePayload<'static, HashMap<String, Vec<MethodRef>>> {
    log::debug!("Handling all refs request");

    let all_refs = context.refs.read().await;
    let formatted: HashMap<String, Vec<MethodRef>> = all_refs
        .iter()
        .map(|(k, v)| (k.to_string(), v.clone()))
        .collect();

    ResponsePayload::success(formatted)
}

async fn rpc_method_handler(
    _params: Params<'static>,
    context: Arc<Crawler>,
    _extensions: Extensions,
) -> ResponsePayload<'static, Vec<MethodRef>> {
    log::debug!("Handling method request");

    let method: Method = match _params.parse() {
        Ok(m) => m,
        Err(e) => return ResponsePayload::error(e),
    };

    let all_refs = context.refs.read().await;

    ResponsePayload::success(all_refs.get(&method).cloned().unwrap_or_default())
}
