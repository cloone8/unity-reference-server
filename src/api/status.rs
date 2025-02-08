use std::sync::Arc;

use jsonrpsee::types::Params;
use jsonrpsee::{Extensions, ResponsePayload};
use serde::{Deserialize, Serialize};

use crate::crawler::Crawler;

pub async fn rpc_status_handler(
    _params: Params<'static>,
    context: Arc<Crawler>,
    _extensions: Extensions,
) -> ResponsePayload<'static, ServerStatus> {
    log::debug!("Handling status request");

    ResponsePayload::success(context.status.read().await.clone())
}

#[derive(Serialize, Deserialize, Clone, Debug)]
pub enum ServerStatus {
    Inactive,
    Initializing,
    Ready,
}
