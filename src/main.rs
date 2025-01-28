use std::sync::Arc;

use args::CliArgs;
use clap::Parser;
use crawler::{Crawler, ServerStatus};
use jsonrpsee::server::Server;
use jsonrpsee::types::Params;
use jsonrpsee::{Extensions, ResponsePayload, RpcModule};
use simplelog::{ConfigBuilder, TermLogger};

mod args;
mod crawler;

#[tokio::main]
async fn main() {
    let args = CliArgs::parse();

    TermLogger::init(
        args.verbosity.into(),
        ConfigBuilder::new().build(),
        simplelog::TerminalMode::Mixed,
        simplelog::ColorChoice::Auto,
    )
    .unwrap();

    log::info!("Watching folder: {}", args.folder.to_string_lossy());
    log::info!("Listening on address and port: {}:{}", args.addr, args.port);

    let full_addr = format!("{}:{}", args.addr, args.port);

    let server = Server::builder().build(full_addr).await.unwrap();

    let crawler = Arc::new(Crawler::new(args.folder).await);
    crawler.start().await;

    let mut module = RpcModule::from_arc(crawler);
    module
        .register_async_method("status", rpc_status_handler)
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
