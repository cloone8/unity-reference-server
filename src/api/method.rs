use core::fmt::Display;
use std::sync::Arc;

use jsonrpsee::types::Params;
use jsonrpsee::{Extensions, ResponsePayload};
use serde::{Deserialize, Serialize};

use crate::crawler::{Crawler, MethodDefinition, Reference};

pub async fn rpc_method_handler(
    params: Params<'static>,
    context: Arc<Crawler>,
    _extensions: Extensions,
) -> ResponsePayload<'static, Vec<MethodResponse>> {
    log::debug!("Handling method request");

    let method: MethodParam = match params.parse() {
        Ok(m) => m,
        Err(e) => return ResponsePayload::error(e),
    };

    let all_refs = context.method_refs.read().await;
    let method_refs = match all_refs.get(&method.into()) {
        Some(r) => r,
        None => return ResponsePayload::success(Vec::new()),
    };

    ResponsePayload::success(method_refs.iter().map(|r| r.clone().into()).collect())
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Deserialize, Serialize)]
pub struct MethodParam {
    pub method_name: String,
    pub method_assembly: String,
    pub method_typename: String,
}

impl From<MethodParam> for MethodDefinition {
    fn from(value: MethodParam) -> Self {
        Self {
            method_name: value.method_name,
            method_assembly: value.method_assembly,
            method_typename: value.method_typename,
        }
    }
}

impl Display for MethodParam {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{}.{}.{}",
            self.method_assembly, self.method_typename, self.method_name
        )
    }
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct MethodResponse {
    pub file: String,
}

impl From<Reference> for MethodResponse {
    fn from(value: Reference) -> Self {
        Self {
            file: value.file.to_string_lossy().to_string(),
        }
    }
}
