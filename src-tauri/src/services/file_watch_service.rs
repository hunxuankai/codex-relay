use crate::error::AppError;
use crate::infrastructure::file_fingerprint::FileSetFingerprint;
use crate::infrastructure::path_service::AppPaths;
use notify::{RecommendedWatcher, RecursiveMode, Watcher};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::mpsc::{self, RecvTimeoutError, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};
use std::time::Duration;

pub const DEFAULT_WATCH_DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum WatchedFileKind {
    Config,
    Auth,
    Providers,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigFilesChanged {
    pub kinds: Vec<WatchedFileKind>,
    pub fingerprints: FileSetFingerprint,
}

pub trait FileWatchEventSink: Send + Sync {
    fn emit(&self, event: ConfigFilesChanged) -> Result<(), AppError>;
}

enum WatchMessage {
    Path(PathBuf),
    Stop,
}

struct WatchWorker {
    sender: Sender<WatchMessage>,
    application_write: Arc<Mutex<Option<FileSetFingerprint>>>,
    thread: Option<JoinHandle<()>>,
}

impl WatchWorker {
    fn start(paths: AppPaths, sink: Arc<dyn FileWatchEventSink>, debounce: Duration) -> Self {
        let (sender, receiver) = mpsc::channel();
        let application_write = Arc::new(Mutex::new(None));
        let worker_application_write = application_write.clone();
        let thread = thread::spawn(move || {
            let mut pending = BTreeSet::new();
            while let Ok(WatchMessage::Path(path)) = receiver.recv() {
                if let Some(kind) = classify_path(&paths, &path) {
                    pending.insert(kind);
                }

                loop {
                    match receiver.recv_timeout(debounce) {
                        Ok(WatchMessage::Path(path)) => {
                            if let Some(kind) = classify_path(&paths, &path) {
                                pending.insert(kind);
                            }
                        }
                        Ok(WatchMessage::Stop) => return,
                        Err(RecvTimeoutError::Timeout) => break,
                        Err(RecvTimeoutError::Disconnected) => return,
                    }
                }

                if pending.is_empty() {
                    continue;
                }
                let fingerprints = match FileSetFingerprint::from_paths(
                    &paths.config_file,
                    &paths.auth_file,
                    &paths.providers_file,
                ) {
                    Ok(fingerprints) => fingerprints,
                    Err(_) => {
                        pending.clear();
                        continue;
                    }
                };
                let suppressed = {
                    let mut expected = worker_application_write
                        .lock()
                        .expect("file-watch application-write lock poisoned");
                    let suppressed = expected.as_ref() == Some(&fingerprints);
                    if expected.is_some() {
                        *expected = None;
                    }
                    suppressed
                };
                if !suppressed {
                    let _ = sink.emit(ConfigFilesChanged {
                        kinds: pending.iter().copied().collect(),
                        fingerprints,
                    });
                }
                pending.clear();
            }
        });

        Self {
            sender,
            application_write,
            thread: Some(thread),
        }
    }

    #[cfg(test)]
    fn notify_path(&self, path: PathBuf) -> Result<(), AppError> {
        self.sender.send(WatchMessage::Path(path)).map_err(|error| {
            AppError::new(
                "FILE_WATCH_STOPPED",
                "配置文件监控已停止。",
                error.to_string(),
            )
        })
    }

    fn mark_application_write(&self, fingerprints: FileSetFingerprint) -> Result<(), AppError> {
        let mut expected = self.application_write.lock().map_err(|_| {
            AppError::new(
                "FILE_WATCH_STATE_FAILED",
                "无法更新配置文件监控状态。",
                "file-watch application-write lock poisoned",
            )
        })?;
        *expected = Some(fingerprints);
        Ok(())
    }
}

impl Drop for WatchWorker {
    fn drop(&mut self) {
        let _ = self.sender.send(WatchMessage::Stop);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

pub struct FileWatchService {
    _watcher: RecommendedWatcher,
    worker: WatchWorker,
}

impl FileWatchService {
    pub fn start(paths: AppPaths, sink: Arc<dyn FileWatchEventSink>) -> Result<Self, AppError> {
        let worker = WatchWorker::start(paths.clone(), sink, DEFAULT_WATCH_DEBOUNCE);
        let sender = worker.sender.clone();
        let mut watcher =
            notify::recommended_watcher(move |event: notify::Result<notify::Event>| {
                if let Ok(event) = event {
                    for path in event.paths {
                        let _ = sender.send(WatchMessage::Path(path));
                    }
                }
            })
            .map_err(|error| {
                AppError::new(
                    "FILE_WATCH_START_FAILED",
                    "无法启动配置文件监控。",
                    error.to_string(),
                )
            })?;
        watcher
            .watch(&paths.codex_home, RecursiveMode::NonRecursive)
            .map_err(|error| {
                AppError::new(
                    "FILE_WATCH_START_FAILED",
                    "无法监控 Codex 配置目录。",
                    error.to_string(),
                )
            })?;
        watcher
            .watch(&paths.app_data_dir, RecursiveMode::NonRecursive)
            .map_err(|error| {
                AppError::new(
                    "FILE_WATCH_START_FAILED",
                    "无法监控 Codex Relay 数据目录。",
                    error.to_string(),
                )
            })?;

        Ok(Self {
            _watcher: watcher,
            worker,
        })
    }

    pub fn mark_application_write(&self, fingerprints: FileSetFingerprint) -> Result<(), AppError> {
        self.worker.mark_application_write(fingerprints)
    }
}

fn classify_path(paths: &AppPaths, changed: &Path) -> Option<WatchedFileKind> {
    if paths_equal(changed, &paths.config_file) {
        Some(WatchedFileKind::Config)
    } else if paths_equal(changed, &paths.auth_file) {
        Some(WatchedFileKind::Auth)
    } else if paths_equal(changed, &paths.providers_file) {
        Some(WatchedFileKind::Providers)
    } else {
        None
    }
}

fn paths_equal(left: &Path, right: &Path) -> bool {
    left.to_string_lossy()
        .replace('/', "\\")
        .eq_ignore_ascii_case(&right.to_string_lossy().replace('/', "\\"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::path_service::AppPaths;
    use std::fs;
    use std::sync::Arc;
    use std::sync::mpsc;
    use std::time::Duration;

    struct ChannelSink {
        sender: mpsc::Sender<ConfigFilesChanged>,
    }

    impl FileWatchEventSink for ChannelSink {
        fn emit(&self, event: ConfigFilesChanged) -> Result<(), AppError> {
            self.sender.send(event).map_err(|error| {
                AppError::new(
                    "TEST_EVENT_SEND_FAILED",
                    "测试事件发送失败。",
                    error.to_string(),
                )
            })
        }
    }

    fn create_paths(directory: &tempfile::TempDir) -> AppPaths {
        let paths = AppPaths::for_test(
            directory.path().join("codex"),
            directory.path().join("app-data"),
        )
        .unwrap();
        fs::create_dir_all(&paths.codex_home).unwrap();
        fs::create_dir_all(&paths.app_data_dir).unwrap();
        paths
    }

    #[test]
    fn burst_changes_are_debounced_into_one_typed_event() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        fs::write(&paths.config_file, "model_provider = \"provider-a\"\n").unwrap();
        fs::write(
            &paths.auth_file,
            "{\"OPENAI_API_KEY\":\"test-key-a-not-real\"}\n",
        )
        .unwrap();
        let (sender, receiver) = mpsc::channel();
        let worker = WatchWorker::start(
            paths.clone(),
            Arc::new(ChannelSink { sender }),
            Duration::from_millis(20),
        );

        worker.notify_path(paths.config_file.clone()).unwrap();
        worker.notify_path(paths.auth_file.clone()).unwrap();
        worker.notify_path(paths.config_file.clone()).unwrap();

        let event = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(event.kinds.len(), 2);
        assert!(event.kinds.contains(&WatchedFileKind::Config));
        assert!(event.kinds.contains(&WatchedFileKind::Auth));
        assert!(receiver.recv_timeout(Duration::from_millis(80)).is_err());
    }

    #[test]
    fn application_write_fingerprint_is_suppressed_once() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        fs::write(&paths.config_file, "model_provider = \"provider-a\"\n").unwrap();
        let fingerprints = FileSetFingerprint::from_paths(
            &paths.config_file,
            &paths.auth_file,
            &paths.providers_file,
        )
        .unwrap();
        let (sender, receiver) = mpsc::channel();
        let worker = WatchWorker::start(
            paths.clone(),
            Arc::new(ChannelSink { sender }),
            Duration::from_millis(20),
        );
        worker.mark_application_write(fingerprints).unwrap();

        worker.notify_path(paths.config_file.clone()).unwrap();
        assert!(receiver.recv_timeout(Duration::from_millis(100)).is_err());

        fs::write(&paths.config_file, "model_provider = \"external\"\n").unwrap();
        worker.notify_path(paths.config_file.clone()).unwrap();
        let event = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        assert_eq!(event.kinds, vec![WatchedFileKind::Config]);
    }

    #[test]
    fn provider_secret_change_event_never_contains_file_contents() {
        let directory = tempfile::tempdir().unwrap();
        let paths = create_paths(&directory);
        fs::write(
            &paths.providers_file,
            "{\"version\":1,\"providers\":{\"provider-a\":{\"apiKey\":\"test-key-a-not-real\"}}}\n",
        )
        .unwrap();
        let (sender, receiver) = mpsc::channel();
        let worker = WatchWorker::start(
            paths.clone(),
            Arc::new(ChannelSink { sender }),
            Duration::from_millis(20),
        );

        worker.notify_path(paths.providers_file.clone()).unwrap();
        let event = receiver.recv_timeout(Duration::from_secs(1)).unwrap();
        let json = serde_json::to_string(&event).unwrap();

        assert_eq!(event.kinds, vec![WatchedFileKind::Providers]);
        assert!(!json.contains("test-key-a-not-real"));
        assert!(!json.contains("apiKey"));
        assert!(!json.contains("OPENAI_API_KEY"));
    }
}
