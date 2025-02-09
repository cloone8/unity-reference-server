use notify::{Config, Event, PollWatcher, RecommendedWatcher, RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::mpsc::{channel, Receiver};
use std::sync::Arc;
use tokio::runtime::Handle;

use crate::crawler::Crawler;

/// Async, futures channel based event watching
pub fn start_watch(crawler: Arc<Crawler>, path: &Path, runtime: Handle) {
    let buf = path.to_path_buf();
    std::thread::spawn(|| watch(crawler, buf, runtime));
}

fn watch(crawler: Arc<Crawler>, path: PathBuf, runtime: Handle) {
    let (mut watcher, rx) = match make_watcher() {
        Ok((w, rx)) => (w, rx),
        Err(e) => {
            log::error!(
                "Error while constructing filesystem watcher, no changes will be processed: {}",
                e
            );
            return;
        }
    };

    // Add a path to be watched. All files and directories at that path and
    // below will be monitored for changes.
    if let Err(e) = watcher.watch(path.as_ref(), RecursiveMode::Recursive) {
        log::error!(
            "Error starting the constructed filesystem watcher, no changes will be processed: {}",
            e
        );
        return;
    };

    for res in rx {
        match res {
            Ok(event) => {
                log::trace!("Filesystem event: {:#?}", event);

                let cloned_crawler = crawler.clone();

                let handle = runtime.spawn(async {
                    handle_event(cloned_crawler, event).await;
                });

                futures::executor::block_on(async {
                    handle.await.unwrap();
                });
            }
            Err(e) => {
                log::warn!("Filesystem watch error: {}", e);
            }
        }
    }
}

fn make_watcher() -> notify::Result<(Box<dyn Watcher>, Receiver<notify::Result<Event>>)> {
    let (tx, rx) = channel();

    let config = Config::default();

    // Automatically select the best implementation for your platform.
    // You can also access each implementation directly e.g. INotifyWatcher.
    let watcher = match RecommendedWatcher::new(tx.clone(), config) {
        Ok(w) => Box::new(w) as Box<dyn Watcher>,
        Err(e) => {
            log::warn!("Could not construct the recommended filesystem watcher, trying fallback poll watcher: {}", e);

            match PollWatcher::new(tx, config) {
                Ok(w) => Box::new(w) as Box<dyn Watcher>,
                Err(e) => {
                    log::error!("Could not construct fallback filesystem watcher: {}", e);
                    return Err(e);
                }
            }
        }
    };

    Ok((watcher, rx))
}

async fn handle_event(crawler: Arc<Crawler>, event: notify::Event) {}
