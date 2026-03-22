use std::process::{Command, Child, Stdio};
use std::io::{BufRead, BufReader};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;

use crate::log::{LogSource, LogEntry, LogSourceInfo};
use crate::log::parser::parse_log_line;

/// 本地命令日志源
pub struct CommandSource {
    command: String,
    args: Vec<String>,
    child: Option<Child>,
    running: Arc<AtomicBool>,
    entries: crossbeam_channel::Receiver<LogEntry>,
    sender: crossbeam_channel::Sender<LogEntry>,
}

impl CommandSource {
    pub fn new(command: String) -> Self {
        let (sender, entries) = crossbeam_channel::unbounded();
        Self {
            command,
            args: Vec::new(),
            child: None,
            running: Arc::new(AtomicBool::new(false)),
            entries,
            sender,
        }
    }
    
    pub fn with_args(command: String, args: Vec<String>) -> Self {
        let mut source = Self::new(command);
        source.args = args;
        source
    }
}

impl LogSource for CommandSource {
    fn start(&mut self) -> anyhow::Result<()> {
        let running = self.running.clone();
        running.store(true, Ordering::SeqCst);
        
        let mut child = Command::new(&self.command)
            .args(&self.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;
        
        let stdout = child.stdout.take().expect("Failed to capture stdout");
        let stderr = child.stderr.take().expect("Failed to capture stderr");
        
        let sender = self.sender.clone();
        let source_info = LogSourceInfo::Local(self.command.clone());
        
        // 启动 stdout 读取线程
        let running_clone = running.clone();
        let sender_clone = sender.clone();
        let source_info_clone = source_info.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stdout);
            for line in reader.lines() {
                if !running_clone.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(line) = line {
                    if let Some(entry) = parse_log_line(&line, source_info_clone.clone()) {
                        let _ = sender_clone.send(entry);
                    }
                }
            }
        });
        
        // 启动 stderr 读取线程
        let sender_clone = sender.clone();
        thread::spawn(move || {
            let reader = BufReader::new(stderr);
            for line in reader.lines() {
                if !running.load(Ordering::SeqCst) {
                    break;
                }
                if let Ok(line) = line {
                    if let Some(entry) = parse_log_line(&line, source_info.clone()) {
                        let _ = sender_clone.send(entry);
                    }
                }
            }
        });
        
        self.child = Some(child);
        Ok(())
    }
    
    fn stop(&mut self) -> anyhow::Result<()> {
        self.running.store(false, Ordering::SeqCst);
        if let Some(ref mut child) = self.child {
            let _ = child.kill();
        }
        self.child = None;
        Ok(())
    }
    
    fn try_recv(&mut self) -> Option<LogEntry> {
        self.entries.try_recv().ok()
    }
    
    fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}