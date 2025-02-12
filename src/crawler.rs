use core::fmt::Display;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::exit;
use std::sync::{Arc, LazyLock};

use regex::Regex;
use saphyr::Yaml;
use tokio::fs::DirEntry;
use tokio::io::{self, AsyncReadExt};
use tokio::sync::RwLock;
use tokio::task::JoinSet;
use tokio::time::Instant;

use crate::api::status::StatusResponse;
use crate::yamlparser::search_yaml_doc;

static UNITY_STRIPPED_REGEX: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(--- .* .*) stripped").unwrap());

#[derive(Debug, Clone)]
pub struct Crawler {
    dir: PathBuf,
    pub status: Arc<RwLock<StatusResponse>>,
    pub method_refs: Arc<RwLock<HashMap<MethodDefinition, Vec<Reference>>>>,
    pub object_refs: Arc<RwLock<HashMap<ObjectDefinition, Vec<Reference>>>>,
}

#[derive(Debug, Clone)]
pub struct ArcRefSet {
    pub methods: Arc<RwLock<HashMap<MethodDefinition, Vec<Reference>>>>,
    pub objects: Arc<RwLock<HashMap<ObjectDefinition, Vec<Reference>>>>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct MethodDefinition {
    pub method_name: String,
    pub method_assembly: String,
    pub method_typename: String,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq)]
pub struct ObjectDefinition {
    pub guid: String,
}

impl Crawler {
    pub async fn new(dir: impl AsRef<Path>) -> Self {
        Self {
            dir: dir.as_ref().to_path_buf(),
            status: Arc::new(RwLock::const_new(StatusResponse::Inactive)),
            method_refs: Arc::new(RwLock::const_new(HashMap::default())),
            object_refs: Arc::new(RwLock::const_new(HashMap::default())),
        }
    }

    pub async fn start(&self) {
        if !matches!(*self.status.read().await, StatusResponse::Inactive) {
            return;
        }

        let mut status = self.status.write().await;

        if !matches!(*status, StatusResponse::Inactive) {
            return;
        }

        *status = StatusResponse::Initializing;

        std::mem::drop(status);

        let status_arc = self.status.clone();

        let refset = self.make_refset();

        let dir = self.dir.clone();

        tokio::spawn(async move {
            log::debug!("Starting crawler");
            let start_time = Instant::now();

            match crawl_dir(&dir, refset).await {
                Ok(()) => {
                    *status_arc.write().await = StatusResponse::Ready;
                    log::info!(
                        "Crawler done after {}s",
                        Instant::now().duration_since(start_time).as_secs_f32()
                    );
                }
                Err(e) => {
                    log::error!("Error starting crawler: {}", e);
                    exit(1);
                }
            }
        });
    }

    fn make_refset(&self) -> ArcRefSet {
        ArcRefSet {
            methods: self.method_refs.clone(),
            objects: self.object_refs.clone(),
        }
    }
}

const EXTENSIONS: &[&str] = &["unity", "prefab"];

async fn crawl_dir(dir: &Path, refs: ArcRefSet) -> io::Result<()> {
    log::debug!("Crawling directory {}", dir.to_string_lossy());

    let mut files = tokio::fs::read_dir(dir).await?;

    let mut tasks = JoinSet::new();

    while let Some(item) = files.next_entry().await? {
        crawl_dir_entry(item, &mut tasks, refs.clone());
    }

    tasks.join_all().await;

    Ok(())
}

fn crawl_dir_entry(item: DirEntry, join_set: &mut JoinSet<()>, refs: ArcRefSet) {
    join_set.spawn(async move {
        let item_type = item.file_type().await.unwrap();

        if item_type.is_dir() {
            if let Err(e) = crawl_dir(&item.path(), refs).await {
                log::warn!(
                    "Error while trying to crawl subdirectory {}: {}",
                    item.path().to_string_lossy(),
                    e
                );
            }
        } else if item_type.is_file() {
            handle_file(&item.path(), refs).await;
        } else if item_type.is_symlink() {
            log::warn!("Skipping symlink at {}", item.path().to_string_lossy());
        } else {
            log::warn!(
                "Unknown filetype, cannot handle it (or any subdirectories): {:#?}",
                item_type
            );
        }
    });
}

async fn handle_file(file: &Path, refs: ArcRefSet) {
    if let Some(extension) = file.extension().and_then(|ext| ext.to_str()) {
        if EXTENSIONS.contains(&extension) {
            log::debug!("Found possible file: {}", file.to_string_lossy());
            let parsed = match read_file_to_yaml(file).await {
                Ok(p) => p,
                Err(e) => {
                    log::warn!(
                        "Error reading or parsing file {}: {}",
                        file.to_string_lossy(),
                        e
                    );
                    return;
                }
            };

            log::debug!("Parsed {} succesfully", file.to_string_lossy());

            let mut document_tasks = JoinSet::new();

            let file_arc = Arc::new(file.to_path_buf());

            for doc in parsed {
                let file_cloned = file_arc.clone();
                let refs_cloned = refs.clone();
                document_tasks.spawn(async move {
                    log::trace!(
                        "Searching document in file {}",
                        file_cloned.to_string_lossy()
                    );
                    search_yaml_doc(&doc, &refs_cloned, &file_cloned).await;
                });
            }

            document_tasks.join_all().await;
        }
    }
}

#[derive(Debug)]
enum ReadErr {
    Io(io::Error),
    Yaml(saphyr::ScanError),
}

impl Display for ReadErr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ReadErr::Io(error) => error.fmt(f),
            ReadErr::Yaml(error) => error.fmt(f),
        }
    }
}

impl From<io::Error> for ReadErr {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}

impl From<saphyr::ScanError> for ReadErr {
    fn from(value: saphyr::ScanError) -> Self {
        Self::Yaml(value)
    }
}

async fn read_file_to_yaml(file: &Path) -> Result<Vec<Yaml>, ReadErr> {
    let mut open_file = tokio::fs::File::open(file).await?;

    let mut content = String::new();

    open_file.read_to_string(&mut content).await?;

    // Dirty hack to get around Unity's broken YAML implementation before I find
    // a proper solution
    let cleaned = UNITY_STRIPPED_REGEX.replace_all(&content, "$1");

    let mut parser = saphyr_parser::Parser::new_from_str(&cleaned).keep_tags(true);
    Ok(Yaml::load_from_parser(&mut parser)?)
}

#[derive(Debug, Clone)]
pub struct Reference {
    /// Which file? (`/MyProject/MyScene.unity`)
    pub file: PathBuf,

    /// While line in the file?
    pub line: Option<usize>,

    /// The human readable name of the referencing asset (`MyScene`)
    pub asset: Option<String>,

    /// The human readable path to the referencing object within the asset (`GameObject A`-> `GameObject B` -> etc.)
    pub object: Option<Vec<String>>,
}
