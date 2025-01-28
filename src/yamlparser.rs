use std::collections::HashMap;
use std::path::Path;

use saphyr::Yaml;
use tokio::sync::RwLock;

use crate::crawler::{Method, MethodRef};

pub async fn search_yaml_doc(
    doc: &Yaml,
    refs: &RwLock<HashMap<Method, Vec<MethodRef>>>,
    origin_file: &Path,
) {
    if !matches!(doc, Yaml::Hash(_)) {
        log::warn!("Unknown Unity YAML root document type");
        return;
    }

    let as_mono = &doc["MonoBehaviour"];
    if !matches!(as_mono, Yaml::BadValue) {
        search_monobehaviour(as_mono, refs, origin_file).await;
    }
}

async fn search_monobehaviour(
    mono: &Yaml,
    refs: &RwLock<HashMap<Method, Vec<MethodRef>>>,
    origin_file: &Path,
) {
    assert!(
        matches!(mono, Yaml::Hash(_)),
        "MonoBehaviour YAML node can only be a hashmap"
    );

    let my_method_ref = MethodRef {
        file: origin_file.to_path_buf(),
    };

    search_mono_fields_recursive(mono, refs, &my_method_ref).await;
}

async fn search_mono_fields_recursive(
    node: &Yaml,
    refs: &RwLock<HashMap<Method, Vec<MethodRef>>>,
    my_ref: &MethodRef,
) {
    log::trace!("Searching YAML node");

    match node {
        Yaml::Array(yamls) => {
            let futures: Vec<_> = yamls
                .iter()
                .map(|yaml| search_mono_fields_recursive(yaml, refs, my_ref))
                .collect();

            futures::future::join_all(futures).await;
        }
        Yaml::Hash(linked_hash_map) => {
            let futures: Vec<_> = linked_hash_map
                .iter()
                .map(|(key, val)| async move {
                    if key == &Yaml::String(String::from("m_PersistentCalls")) {
                        let found_method_calls = parse_persistent_calls(val);

                        let mut refs_locked = refs.write().await;

                        for found_method_call in found_method_calls {
                            refs_locked
                                .entry(found_method_call)
                                .or_default()
                                .push(my_ref.clone());
                        }
                    } else {
                        search_mono_fields_recursive(val, refs, my_ref).await;
                    }
                })
                .collect();

            futures::future::join_all(futures).await;
        }
        _ => (),
    }
}

fn parse_persistent_calls(persistent_calls: &Yaml) -> Vec<Method> {
    log::trace!("Found persistent call: {:#?}", persistent_calls);

    if let Yaml::Array(targets) = &persistent_calls["m_Calls"] {
        targets.iter().filter_map(parse_call).collect()
    } else {
        Vec::new()
    }
}

fn parse_call(call: &Yaml) -> Option<Method> {
    let method_target = &call["m_MethodName"];
    let target_assembly_type = &call["m_TargetAssemblyTypeName"];

    if let Yaml::String(method_name) = method_target {
        if let Yaml::String(method_assembly_type) = target_assembly_type {
            let (class, assembly) = method_assembly_type.split_once(", ").expect("REMOVE THIS");

            let found_method_call = Method {
                method_name: method_name.clone(),
                method_assembly: assembly.to_owned(),
                method_typename: class.to_owned(),
            };

            log::trace!("Found call to {:#?}", found_method_call);

            return Some(found_method_call);
        }
    }

    None
}
