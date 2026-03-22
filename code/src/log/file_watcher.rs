use std::fs::File;
use std::io::{BufRead, BufReader, Seek, SeekFrom};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

use crate::log::{LogSource, LogEntry, LogSourceInfo};
use crate::log::parser::parse_log_line;

/// 文件跟踪日志源
pub struct FileWatchSource {
    path: PathBuf,
    running: Arc<AtomicBool>,
    entries: crossbeam_channel::Receiver<LogEntry>,
    sender: crossbeam_channel::Sender<LogEntry>,
}

impl FileWatchSource {
    pub fn new(path: PathBuf) -> Self {
        let (sender, entries) = crossbeam_channel::unbounded();
        Self {
            path,
            running: Arc::new(AtomicBool::new(false)),
            entries,
            sender,
        }
    }

    /// 获取文件路径
    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

impl LogSource for FileWatchSource {
    fn start(&mut self) -> anyhow::Result<()> {
        let running = self.running.clone();
        running.store(true, Ordering::SeqCst);

        let path = self.path.clone();
        let sender = self.sender.clone();
        let source_info = LogSourceInfo::File(path.to_string_lossy().to_string());

        thread::spawn(move || {
            let mut file = match File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Failed to open file {:?}: {}", path, e);
                    return;
                }
            };

            // 跳到文件末尾，只读取新内容
            let mut last_pos = match file.seek(SeekFrom::End(0)) {
                Ok(pos) => pos,
                Err(_) => 0,
            };

            while running.load(Ordering::SeqCst) {
                // 检查文件大小
                let current_size = match file.metadata() {
                    Ok(m) => m.len(),
                    Err(_) => {
                        thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                };

                if current_size > last_pos {
                    // 有新内容，读取
                    if let Err(_) = file.seek(SeekFrom::Start(last_pos)) {
                        continue;
                    }

                    let reader = BufReader::new(&file);
                    for line in reader.lines() {
                        if !running.load(Ordering::SeqCst) {
                            break;
                        }

                        if let Ok(line) = line {
                            if let Some(entry) = parse_log_line(&line, source_info.clone()) {
                                let _ = sender.send(entry);
                            }
                            last_pos += line.len() as u64 + 1; // +1 for newline
                        }
                    }
                } else if current_size < last_pos {
                    // 文件被截断或重新创建，从头开始
                    if let Ok(pos) = file.seek(SeekFrom::Start(0)) {
                        last_pos = pos;
                    }
                }

                thread::sleep(Duration::from_millis(100));
            }
        });

        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn try_recv(&mut self) -> Option<LogEntry> {
        self.entries.try_recv().ok()
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// 静态文件日志源（一次性读取整个文件）
pub struct FileSource {
    path: PathBuf,
    running: Arc<AtomicBool>,
    entries: crossbeam_channel::Receiver<LogEntry>,
    sender: crossbeam_channel::Sender<LogEntry>,
}

impl FileSource {
    pub fn new(path: PathBuf) -> Self {
        let (sender, entries) = crossbeam_channel::unbounded();
        Self {
            path,
            running: Arc::new(AtomicBool::new(false)),
            entries,
            sender,
        }
    }
}

impl LogSource for FileSource {
    fn start(&mut self) -> anyhow::Result<()> {
        let running = self.running.clone();
        running.store(true, Ordering::SeqCst);

        let path = self.path.clone();
        let sender = self.sender.clone();
        let source_info = LogSourceInfo::File(path.to_string_lossy().to_string());

        thread::spawn(move || {
            let file = match File::open(&path) {
                Ok(f) => f,
                Err(e) => {
                    eprintln!("Failed to open file {:?}: {}", path, e);
                    return;
                }
            };

            let reader = BufReader::new(file);
            for line in reader.lines() {
                if !running.load(Ordering::SeqCst) {
                    break;
                }

                if let Ok(line) = line {
                    if let Some(entry) = parse_log_line(&line, source_info.clone()) {
                        let _ = sender.send(entry);
                    }
                }
            }

            // 读取完成，停止运行
            running.store(false, Ordering::SeqCst);
        });

        Ok(())
    }

    fn stop(&mut self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        Ok(())
    }

    fn try_recv(&mut self) -> Option<LogEntry> {
        self.entries.try_recv().ok()
    }

    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}
