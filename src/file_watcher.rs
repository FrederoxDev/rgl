use anyhow::{Context, Result};
use notify::{recommended_watcher, Event, RecommendedWatcher, RecursiveMode, Watcher};
use smol::{channel, Timer};
use std::{path::Path, time::Duration};

pub struct FileWatcher {
    rx: channel::Receiver<()>,
    watcher: RecommendedWatcher,
}

impl FileWatcher {
    pub fn new() -> Result<Self> {
        let (tx, rx) = channel::unbounded();
        let watcher = recommended_watcher(move |event: notify::Result<Event>| {
            if let Ok(e) = event {
                if !e.kind.is_access() && !e.kind.is_other() {
                    let _ = tx.send_blocking(());
                }
            }
        })
        .context("Failed to create file watcher")?;

        Ok(Self { rx, watcher })
    }

    pub fn add_path(&mut self, path: impl AsRef<Path>) -> Result<()> {
        let path = path.as_ref();
        self.watcher
            .watch(path, RecursiveMode::Recursive)
            .context(format!(
                "Failed to watch directory\n\
                 <yellow> >></> Path: {}",
                path.display()
            ))
    }

    pub async fn wait_changes(&self) {
        let _ = self.rx.recv().await;
        loop {
            let stop = smol::future::race(
                async {
                    let _ = self.rx.recv().await;
                    false
                },
                async {
                    Timer::after(Duration::from_millis(100)).await;
                    true
                },
            )
            .await;
            if stop {
                break;
            }
        }
    }
}
