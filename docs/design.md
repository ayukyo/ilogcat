# iLogCat - 技术设计

## 技术选型

### 核心技术栈

| 组件 | 技术选型 | 理由 |
|------|----------|------|
| **编程语言** | Rust | 高性能、内存安全、跨平台 |
| **GUI 框架** | GTK4 | Linux 原生、现代化、性能好 |
| **SSH 库** | ssh2-rs | Rust SSH 实现，功能完整 |
| **配置格式** | TOML | Rust 生态标准，易读易写 |
| **正则引擎** | regex | Rust 官方正则库 |
| **异步运行时** | tokio | 高性能异步 |

### 依赖库

```toml
[dependencies]
gtk4 = "0.7"
ssh2 = "0.9"
regex = "1.10"
tokio = { version = "1.35", features = ["full"] }
serde = { version = "1.0", features = ["derive"] }
toml = "0.8"
chrono = "0.4"
anyhow = "1.0"
thiserror = "1.0"
```

## 架构设计

### 模块划分

```
src/
├── main.rs              # 程序入口
├── app.rs               # 应用主逻辑
├── config.rs            # 配置管理
├── ui/
│   ├── mod.rs
│   ├── window.rs        # 主窗口
│   ├── log_view.rs      # 日志显示组件
│   ├── filter_bar.rs    # 过滤工具栏
│   ├── device_selector.rs # 设备选择器
│   └── dialogs.rs       # 对话框
├── log/
│   ├── mod.rs
│   ├── source.rs        # 日志源 trait
│   ├── local.rs         # 本地日志源
│   ├── file_watcher.rs  # 文件跟踪
│   ├── remote.rs        # SSH 远程日志源
│   └── parser.rs        # 日志解析
├── filter/
│   ├── mod.rs
│   ├── keyword.rs       # 关键字过滤
│   ├── level.rs         # 级别过滤
│   └── regex_filter.rs  # 正则过滤
└── ssh/
    ├── mod.rs
    ├── client.rs        # SSH 客户端
    └── config.rs        # SSH 配置
```

### 核心数据结构

```rust
/// 日志条目
pub struct LogEntry {
    pub timestamp: DateTime<Local>,
    pub level: LogLevel,
    pub tag: String,
    pub message: String,
    pub source: LogSource,
}

/// 日志级别
pub enum LogLevel {
    Verbose,
    Debug,
    Info,
    Warn,
    Error,
    Fatal,
}

/// 日志源
pub enum LogSource {
    Local(String),      // 本地命令输出
    File(String),       // 文件跟踪
    Remote(RemoteInfo), // SSH 远程
}

/// 过滤条件
pub struct Filter {
    pub keywords: Vec<KeywordFilter>,
    pub levels: HashSet<LogLevel>,
    pub regex: Option<Regex>,
}

pub struct KeywordFilter {
    pub text: String,
    pub case_sensitive: bool,
    pub highlight_color: Color,
}

/// SSH 配置
pub struct SshConfig {
    pub name: String,
    pub host: String,
    pub port: u16,
    pub username: String,
    pub auth: AuthMethod,
}

pub enum AuthMethod {
    Password(String),
    KeyFile(PathBuf),
}
```

### 日志源抽象

```rust
/// 日志源 trait
#[async_trait]
pub trait LogSource {
    /// 开始获取日志
    async fn start(&mut self) -> Result<()>;
    
    /// 停止获取日志
    async fn stop(&mut self) -> Result<()>;
    
    /// 接收日志条目
    async fn recv(&mut self) -> Option<LogEntry>;
    
    /// 是否正在运行
    fn is_running(&self) -> bool;
}

/// 本地命令日志源
pub struct CommandSource {
    command: String,
    child: Option<Child>,
    output: Option<BoxStream<String>>,
}

/// 文件跟踪日志源
pub struct FileWatchSource {
    path: PathBuf,
    watcher: Option<RecommendedWatcher>,
}

/// SSH 远程日志源
pub struct SshSource {
    config: SshConfig,
    session: Option<Session>,
    channel: Option<Channel>,
}
```

### UI 组件设计

```rust
/// 主窗口
pub struct MainWindow {
    // GTK 组件
    window: ApplicationWindow,
    log_view: LogView,
    filter_bar: FilterBar,
    device_selector: DeviceSelector,
    status_bar: Statusbar,
    
    // 状态
    current_source: Option<Box<dyn LogSource>>,
    filter: Filter,
    log_buffer: Vec<LogEntry>,
}

/// 日志显示视图
pub struct LogView {
    text_view: TextView,
    buffer: TextBuffer,
    tags: HashMap<LogLevel, TextTag>,
}

/// 过滤工具栏
pub struct FilterBar {
    search_entry: SearchEntry,
    level_combo: ComboBoxText,
    add_filter_btn: Button,
    clear_btn: Button,
    filters: Vec<FilterWidget>,
}
```

## 数据流

```
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  日志源      │────▶│  日志解析    │────▶│  过滤器      │
│ (local/ssh)  │     │  (parser)    │     │  (filter)    │
└──────────────┘     └──────────────┘     └──────────────┘
                                                 │
                                                 ▼
┌──────────────┐     ┌──────────────┐     ┌──────────────┐
│  配置存储    │◀────│  UI 组件     │◀────│  日志缓冲    │
│  (config)    │     │  (GTK)       │     │  (buffer)    │
└──────────────┘     └──────────────┘     └──────────────┘
```

## 配置文件设计

```toml
# ~/.config/ilogcat/config.toml

[general]
default_log_level = "Info"
max_log_lines = 100000
auto_scroll = true

[ui]
font = "Monospace 12"
theme = "dark"

[colors]
verbose = "#808080"
debug = "#0000FF"
info = "#00FF00"
warn = "#FFA500"
error = "#FF0000"
fatal = "#FF0000:bold"

[[ssh_servers]]
name = "production"
host = "192.168.1.100"
port = 22
username = "admin"
key_file = "~/.ssh/id_rsa"

[[ssh_servers]]
name = "staging"
host = "192.168.1.200"
port = 22
username = "dev"
# 密码不保存，运行时输入

[[saved_filters]]
name = "错误日志"
keywords = ["ERROR", "FATAL"]
logic = "OR"
levels = ["Error", "Fatal"]

[[saved_filters]]
name = "应用日志"
keywords = ["MyApp"]
logic = "AND"
levels = ["Verbose", "Debug", "Info", "Warn", "Error", "Fatal"]
```

## 快捷键设计

| 快捷键 | 功能 |
|--------|------|
| `Ctrl+F` | 聚焦搜索框 |
| `Ctrl+L` | 清空日志 |
| `Ctrl+S` | 暂停/恢复日志 |
| `Ctrl+Shift+F` | 添加过滤条件 |
| `Ctrl+1-6` | 切换日志级别 |
| `Ctrl+Q` | 退出 |
| `F5` | 刷新日志源 |
| `Ctrl+Shift+S` | 保存日志到文件 |