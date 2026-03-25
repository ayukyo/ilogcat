# iLogCat

A logcat-like log viewer for Linux, inspired by Android Studio Logcat.

![iLogCat Screenshot](docs/screenshot.png)

## Features

- 🎨 **Color-coded log levels** - Different colors for VERBOSE, DEBUG, INFO, WARN, ERROR, FATAL
- 🔍 **Keyword filtering** - Filter by single or multiple keywords with regex support
- 📋 **Multiple log sources** - Execute commands, watch files, or connect via SSH
- 🔌 **SSH remote support** - View remote logs with the same filtering capabilities
- 📑 **Multi-tab support** - Open multiple log windows in tabs with clear source labels
- ⚙️ **Custom log level keywords** - Define your own keywords for log level detection
- 💾 **Settings export/import** - Backup and share your configuration
- 🌙 **Dark/Light theme** - Switch between themes
- 🌐 **Multi-language** - Chinese/English support
- 💻 **Native Linux GUI** - Built with GTK4 for a native experience

## Installation

### System Requirements

- **OS:** Linux (Ubuntu 22.04+, Debian 12+, Fedora 36+, Arch Linux)
- **GLIBC:** 2.35 or later
- **Runtime:** GTK4, libssh2

### Prerequisites

**Ubuntu/Debian:**
```bash
sudo apt install libgtk-4-1 libssh2-1
```

**Fedora:**
```bash
sudo dnf install gtk4 libssh2
```

**Arch Linux:**
```bash
sudo pacman -S gtk4 libssh2
```

### From Binary (Recommended)

Download the latest release from [GitHub Releases](https://github.com/ayukyo/ilogcat/releases).

```bash
# Download latest version
wget https://github.com/ayukyo/ilogcat/releases/latest/download/ilogcat_0.2.0_amd64.deb
sudo dpkg -i ilogcat_0.2.0_amd64.deb
sudo apt-get install -f  # Install dependencies if needed
```

### From Source

```bash
# Install build dependencies (Ubuntu/Debian)
sudo apt install libgtk-4-dev libssh2-1-dev pkg-config build-essential

# Clone and build
git clone https://github.com/ayukyo/ilogcat.git
cd ilogcat/code
cargo build --release

# Install
sudo cp target/release/ilogcat /usr/local/bin/
```

### Using Install Script

```bash
git clone https://github.com/ayukyo/ilogcat.git
cd ilogcat
sudo ./install.sh
```

## Usage

### Launch iLogCat

```bash
# From terminal
ilogcat

# Or search "iLogCat" in your application menu
```

### Log Sources

1. **Local: dmesg** - View kernel messages
2. **Local: journalctl** - View systemd journal
3. **File...** - Watch a local log file
4. **SSH...** - Connect to remote server
5. **SSH Command...** - Execute custom command on saved SSH server

### Keyboard Shortcuts

| Shortcut | Action |
|----------|--------|
| `Ctrl+T` | New tab |
| `Ctrl+W` | Close current tab |
| `Ctrl+Tab` | Next tab |
| `Ctrl+Shift+Tab` | Previous tab |
| `Ctrl+L` | Clear logs |
| `Ctrl+S` | Pause/Resume |
| `Tab` | Auto-complete command (SSH) |
| `↑` / `↓` | Navigate command history |

### SSH Terminal Mode

After connecting to an SSH server, you can execute commands like a terminal:

- **TAB Completion** - Press Tab to auto-complete commands and paths
- **Multi-line Commands** - Paste multiple commands and execute them one by one
- **Path Tracking** - Automatically track current directory after `cd` commands
- **Command History** - Use ↑/↓ arrows to navigate command history
- **Server Time** - Log timestamps show server time
- **Status Indicator** - Tab shows connection status (🟢 Connected / 🔴 Disconnected)
- **Auto-reconnect** - Automatically reconnect when SSH disconnects

### Configuration

Configuration is stored in `~/.config/com.openclaw.ilogcat/config.toml`

You can export/import settings via the Settings menu.

## Building from Source

### Requirements

- Rust 1.70+
- GTK4 development libraries
- libssh2 development libraries
- pkg-config

### Build

```bash
cd code
cargo build --release
```

The binary will be at `target/release/ilogcat`.

## Development

### Project Structure

```
ilogcat/
├── code/
│   ├── src/
│   │   ├── main.rs          # Application entry
│   │   ├── app_tabs.rs      # Multi-tab UI implementation
│   │   ├── config.rs        # Configuration management
│   │   ├── filter.rs        # Log filtering
│   │   ├── log/             # Log sources
│   │   ├── ssh/             # SSH client
│   │   └── ui/              # UI components
│   └── Cargo.toml
├── docs/
│   ├── requirements.md      # Requirements document
│   ├── design.md            # Technical design
│   └── usage.md             # User guide
└── .github/workflows/
    └── build.yml            # CI/CD configuration
```

## Changelog

### v0.4.0 (2026-03-26)

- **TAB Completion** - SSH command auto-completion like SecureCRT
- **Multi-line Commands** - Paste and execute multiple commands at once
- **Remote Server Time** - Log timestamps use server time for SSH connections
- **Flexible Log Parsing** - Support any number of bracket fields `[tag] [level] [file:line] message`
- **Reset to Default** - Added "Reset Default" button in Theme and Language settings
- Improved log level detection for `[warning]`, `[error]`, `[info]` patterns
- Fixed command input with Windows line endings (`\r\n`)
- Various bug fixes and improvements

### v0.3.0 (2026-03-25)

- Added Trace and Critical log levels
- Improved SSH terminal-like experience with path tracking
- Added command history navigation (Up/Down arrows)
- SSH auto-reconnect when disconnected
- Per-tab filter settings
- Fixed SSH authentication for "none" auth servers
- Fixed Chinese language switching
- Various bug fixes and improvements

### v0.2.3 (2026-03-23)

- Fixed GLIBC compatibility - now works on Ubuntu 22.04+ (GLIBC 2.35+)
- Built in Ubuntu 22.04 container for better compatibility

### v0.2.1 (2026-03-23)

- Fixed desktop icon not showing
- Fixed application not launching from desktop menu

### v0.2.0 (2026-03-23)

- Added multi-tab support
- Added custom log level keywords
- Added SSH remote command execution
- Added settings export/import
- Improved UI and user experience

### v0.1.0 (2026-03-22)

- Initial release
- Basic log viewing functionality
- SSH remote support
- Log level filtering

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- Inspired by Android Studio Logcat
- Built with [GTK4](https://gtk.org/) and [Rust](https://rust-lang.org/)

---

# iLogCat (中文版)

一款类 Logcat 的 Linux 日志查看器，灵感来自 Android Studio Logcat。

![iLogCat 截图](docs/screenshot.png)

## 功能特性

- 🎨 **彩色日志级别** - 为 VERBOSE、DEBUG、INFO、WARN、ERROR、FATAL 等级别显示不同颜色
- 🔍 **关键字过滤** - 支持单个或多个关键字过滤，支持正则表达式
- 📋 **多种日志源** - 执行命令、监控文件、或通过 SSH 连接远程服务器
- 🔌 **SSH 远程支持** - 查看远程日志，支持相同的过滤功能
- 📑 **多标签页支持** - 在标签页中打开多个日志窗口，带清晰的来源标签
- ⚙️ **自定义日志级别关键字** - 定义您自己的日志级别检测关键字
- 💾 **设置导入/导出** - 备份和分享您的配置
- 🌙 **深色/浅色主题** - 在主题之间切换
- 🌐 **多语言支持** - 支持中文/英文
- 💻 **原生 Linux GUI** - 使用 GTK4 构建，提供原生体验

## 安装

### 系统要求

- **操作系统：** Linux (Ubuntu 22.04+, Debian 12+, Fedora 36+, Arch Linux)
- **GLIBC：** 2.35 或更高版本
- **运行时：** GTK4, libssh2

### 依赖安装

**Ubuntu/Debian:**
```bash
sudo apt install libgtk-4-1 libssh2-1
```

**Fedora:**
```bash
sudo dnf install gtk4 libssh2
```

**Arch Linux:**
```bash
sudo pacman -S gtk4 libssh2
```

### 从二进制包安装（推荐）

从 [GitHub Releases](https://github.com/ayukyo/ilogcat/releases) 下载最新版本。

```bash
# 下载最新版本
wget https://github.com/ayukyo/ilogcat/releases/latest/download/ilogcat_0.2.0_amd64.deb
sudo dpkg -i ilogcat_0.2.0_amd64.deb
sudo apt-get install -f  # 安装依赖（如需要）
```

### 从源码编译

```bash
# 安装编译依赖 (Ubuntu/Debian)
sudo apt install libgtk-4-dev libssh2-1-dev pkg-config build-essential

# 克隆并编译
git clone https://github.com/ayukyo/ilogcat.git
cd ilogcat/code
cargo build --release

# 安装
sudo cp target/release/ilogcat /usr/local/bin/
```

### 使用安装脚本

```bash
git clone https://github.com/ayukyo/ilogcat.git
cd ilogcat
sudo ./install.sh
```

## 使用方法

### 启动 iLogCat

```bash
# 从终端启动
ilogcat

# 或在应用程序菜单中搜索 "iLogCat"
```

### 日志来源

1. **本地：dmesg** - 查看内核消息
2. **本地：journalctl** - 查看 systemd 日志
3. **文件...** - 监控本地日志文件
4. **SSH...** - 连接到远程服务器
5. **SSH 命令...** - 在已保存的 SSH 服务器上执行自定义命令

### 键盘快捷键

| 快捷键 | 操作 |
|--------|------|
| `Ctrl+T` | 新建标签页 |
| `Ctrl+W` | 关闭当前标签页 |
| `Ctrl+Tab` | 下一个标签页 |
| `Ctrl+Shift+Tab` | 上一个标签页 |
| `Ctrl+L` | 清空日志 |
| `Ctrl+S` | 暂停/恢复 |
| `Ctrl+F` | 快速搜索 |
| `Tab` | 命令自动补全 (SSH) |
| `↑` / `↓` | 浏览命令历史 |

### SSH 终端模式

连接 SSH 服务器后，可以像使用终端一样执行命令：

- **TAB 补全** - 按 Tab 键自动补全命令和路径
- **多行命令** - 支持粘贴多行命令并逐个执行
- **路径跟踪** - 自动跟踪 `cd` 命令后的当前目录
- **命令历史** - 使用 ↑/↓ 方向键浏览历史命令
- **服务器时间** - 日志时间显示服务器时间
- **状态指示** - 标签页显示连接状态（🟢 已连接 / 🔴 已断开）
- **自动重连** - SSH 断开后自动尝试重连

### 配置

配置文件存储在 `~/.config/com.openclaw.ilogcat/config.toml`

可以通过设置菜单导入/导出设置。

## 从源码构建

### 要求

- Rust 1.70+
- GTK4 开发库
- libssh2 开发库
- pkg-config

### 编译

```bash
cd code
cargo build --release
```

编译后的二进制文件位于 `target/release/ilogcat`。

## 开发

### 项目结构

```
ilogcat/
├── code/
│   ├── src/
│   │   ├── main.rs          # 应用程序入口
│   │   ├── app_tabs.rs      # 多标签页 UI 实现
│   │   ├── config.rs        # 配置管理
│   │   ├── filter/          # 日志过滤
│   │   ├── log/             # 日志源
│   │   ├── ssh/             # SSH 客户端
│   │   └── ui/              # UI 组件
│   └── Cargo.toml
├── docs/
│   ├── requirements.md      # 需求文档
│   ├── design.md            # 技术设计
│   └── usage.md             # 用户指南
└── .github/workflows/
    └── build.yml            # CI/CD 配置
```

## 更新日志

### v0.4.0 (2026-03-26)

- **TAB 补全** - SSH 命令自动补全，类似 SecureCRT
- **多行命令** - 支持粘贴并执行多个命令
- **远程服务器时间** - SSH 连接时日志时间使用服务器时间
- **灵活日志解析** - 支持任意数量的括号字段 `[标签] [级别] [文件:行] 消息`
- **恢复默认** - 主题和语言设置增加"恢复默认"按钮
- 改进 `[warning]`、`[error]`、`[info]` 等日志级别检测
- 修复 Windows 换行符 (`\r\n`) 导致的命令执行问题
- 其他 bug 修复和改进

### v0.3.0 (2026-03-25)

- 新增 Trace 和 Critical 日志级别
- 改进 SSH 终端体验，支持路径跟踪
- 新增命令历史导航（↑/↓ 方向键）
- SSH 断开后自动重连
- 每个标签页独立的过滤设置
- 修复 "none" 认证方式的 SSH 服务器连接问题
- 修复中文语言切换问题
- 其他 bug 修复和改进

### v0.2.3 (2026-03-23)

- 修复 GLIBC 兼容性 - 现在支持 Ubuntu 22.04+ (GLIBC 2.35+)
- 在 Ubuntu 22.04 容器中构建以提高兼容性

### v0.2.1 (2026-03-23)

- 修复桌面图标不显示问题
- 修复无法从桌面菜单启动问题

### v0.2.0 (2026-03-23)

- 新增多标签页支持
- 新增自定义日志级别关键字
- 新增 SSH 远程命令执行
- 新增设置导入/导出
- 改进 UI 和用户体验

### v0.1.0 (2026-03-22)

- 首次发布
- 基本日志查看功能
- SSH 远程支持
- 日志级别过滤

## 许可证

MIT 许可证 - 详情请参阅 [LICENSE](LICENSE) 文件。

## 贡献

欢迎贡献！请随时提交 Pull Request。

## 致谢

- 灵感来自 Android Studio Logcat
- 使用 [GTK4](https://gtk.org/) 和 [Rust](https://rust-lang.org/) 构建