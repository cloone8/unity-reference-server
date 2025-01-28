use std::collections::HashMap;
use std::fmt::format;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::Arc;

use args::CliArgs;
use clap::Parser;
use crawler::{Crawler, Method, MethodRef, ServerStatus};
use jsonrpsee::server::Server;
use jsonrpsee::types::Params;
use jsonrpsee::{Extensions, ResponsePayload, RpcModule};
use saphyr::Yaml;
use simplelog::{ConfigBuilder, TermLogger};
use tokio::sync::RwLock;
use yamlparser::search_yaml_doc;

mod args;
mod crawler;
mod yamlparser;

const TEST: &str = "MonoBehaviour:
  m_ObjectHideFlags: 1
  m_PrefabParentObject: {fileID: 0}
  m_PrefabInternal: {fileID: 100100000}
  m_GameObject: {fileID: 1146717160097598}
  m_Enabled: 1
  m_EditorHideFlags: 0
  m_Script: {fileID: 11500000, guid: 397d688c4b534bd4f9db378be9002e4f, type: 3}
  m_Name:
  m_EditorClassIdentifier:
  targetTag: Points
  delay: 0
  Compound: 0
  waitForGrabRelease: 1
  slave: {fileID: 114754721043387676}
  triggered: 0
  triggerEnterEvent:
    m_PersistentCalls:
      m_Calls:
      - m_Target: {fileID: 0}
        m_MethodName: DecLeftRings
        m_Mode: 1
        m_Arguments:
          m_ObjectArgument: {fileID: 0}
          m_ObjectArgumentAssemblyTypeName: UnityEngine.Object, UnityEngine
          m_IntArgument: 0
          m_FloatArgument: 0
          m_StringArgument:
          m_BoolArgument: 0
        m_CallState: 2
      - m_Target: {fileID: 0}
        m_MethodName: RingInNonMatchingBox
        m_Mode: 0
        m_Arguments:
          m_ObjectArgument: {fileID: 0}
          m_ObjectArgumentAssemblyTypeName: UnityEngine.Object, UnityEngine
          m_IntArgument: 0
          m_FloatArgument: 0
          m_StringArgument:
          m_BoolArgument: 0
        m_CallState: 2
      - m_Target: {fileID: 1483660425}
        m_TargetAssemblyTypeName: ClipAVessel, Simendo.Exercises
        m_MethodName: Done
        m_Mode: 1
        m_Arguments:
          m_ObjectArgument: {fileID: 0}
          m_ObjectArgumentAssemblyTypeName: UnityEngine.Object, UnityEngine
          m_IntArgument: 0
          m_FloatArgument: 0
          m_StringArgument: 
          m_BoolArgument: 0
        m_CallState: 2
      - m_Target: {fileID: 1146717160097598}
        m_MethodName: SetActive
        m_Mode: 6
        m_Arguments:
          m_ObjectArgument: {fileID: 0}
          m_ObjectArgumentAssemblyTypeName: UnityEngine.Object, UnityEngine
          m_IntArgument: 0
          m_FloatArgument: 0
          m_StringArgument:
          m_BoolArgument: 0
        m_CallState: 2
    m_TypeName: Trigger+TriggerEvent, Simendo.Physics, Version=0.0.0.0, Culture=neutral,
      PublicKeyToken=null
  triggerStayEvent:
    m_PersistentCalls:
      m_Calls: []
    m_TypeName: Trigger+TriggerEvent, Simendo.Physics, Version=0.0.0.0, Culture=neutral,
      PublicKeyToken=null
  triggerExitEvent:
    m_PersistentCalls:
      m_Calls: []
    m_TypeName: Trigger+TriggerEvent, Simendo.Physics, Version=0.0.0.0, Culture=neutral,
      PublicKeyToken=null";

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
