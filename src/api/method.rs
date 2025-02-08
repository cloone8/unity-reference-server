use core::fmt::Display;
use std::sync::Arc;

use jsonrpsee::types::Params;
use jsonrpsee::{Extensions, ResponsePayload};
use serde::{Deserialize, Serialize};

use crate::crawler::Crawler;

pub async fn rpc_method_handler(
    params: Params<'static>,
    context: Arc<Crawler>,
    _extensions: Extensions,
) -> ResponsePayload<'static, Vec<MethodRef>> {
    log::debug!("Handling method request");

    let method: Method = match params.parse() {
        Ok(m) => m,
        Err(e) => return ResponsePayload::error(e),
    };

    let all_refs = context.refs.read().await;

    ResponsePayload::success(all_refs.get(&method).cloned().unwrap_or_default())
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub struct Method {
    pub method_name: String,
    pub method_assembly: String,
    pub method_typename: String,
}

impl Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}",
            self.method_assembly, self.method_typename, self.method_name
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MethodRef {
    pub file: String,
}
