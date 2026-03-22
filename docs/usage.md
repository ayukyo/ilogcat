# iLogCat 使用指南

## 简介

iLogCat 是一款模仿 Android Studio Logcat 的 Linux 日志查看工具，支持本地系统日志、文件日志和远程 SSH 日志查看。

## 功能特性

### 日志源支持
- **本地系统日志**：支持 `dmesg` 和 `journalctl`
- **本地文件日志**：支持跟踪任意日志文件（类似 `tail -f`）
- **远程 SSH 日志**：支持通过 SSH 连接远程服务器查看日志

### 日志级别彩色显示
| 级别 | 颜色 | 说明 |
|------|------|------|
| VERBOSE | 灰色 | 详细日志 |
| DEBUG | 蓝色 | 调试日志 |
| INFO | 绿色 | 信息日志 |
| WARN | 橙色 | 警告日志 |
| ERROR | 红色 | 错误日志 |
| FATAL | 红色加粗 | 致命错误 |

### 过滤功能
- **日志级别过滤**：选择最小日志级别，只显示该级别及以上的日志
- **关键字过滤**：支持输入关键字进行过滤
- **实时过滤**：修改过滤条件后自动刷新显示

### 键盘快捷键
| 快捷键 | 功能 |
|--------|------|
| `Ctrl+L` | 清除所有日志 |
| `Ctrl+S` | 暂停/恢复日志流 |

## 安装

### 从源码编译

```bash
# 克隆仓库
git clone https://github.com/ayukyo/ilogcat.git
cd ilogcat

# 编译（需要 Rust 工具链）
cargo build --release

# 运行
./target/release/ilogcat
```

### 系统要求

- Linux 操作系统（主要支持 Ubuntu 20.04+）
- GTK4 运行时库
- Rust 1.70+（编译时需要）

## 使用说明

### 启动应用

```bash
ilogcat
```

### 选择日志源

1. **查看 dmesg 日志**：
   - 在工具栏的 Source 下拉菜单中选择 "Local: dmesg"
   - 自动执行 `dmesg -w --time-format=iso` 命令

2. **查看 journalctl 日志**：
   - 在工具栏的 Source 下拉菜单中选择 "Local: journalctl"
   - 自动执行 `journalctl -f -o short-iso` 命令

3. **查看本地日志文件**：
   - 在工具栏的 Source 下拉菜单中选择 "File..."
   - 在弹出的文件选择对话框中选择要查看的日志文件
   - 自动开始跟踪文件变化

4. **查看远程 SSH 日志**：
   - 在工具栏的 Source 下拉菜单中选择 "SSH..."
   - 在弹出的 SSH 连接对话框中填写：
     - **Name**: 连接名称（用于保存配置）
     - **Host**: 服务器地址（IP 或域名）
     - **Port**: SSH 端口（默认 22）
     - **Username**: 用户名
     - **Password**: 密码
   - 点击 Connect 连接
   - 连接成功后自动执行远程日志命令

### 过滤日志

1. **按日志级别过滤**：
   - 在工具栏的 Min Level 下拉菜单中选择最小日志级别
   - 只显示该级别及以上的日志
   - 例如选择 "Info" 将显示 INFO、WARN、ERROR、FATAL 级别的日志

2. **关键字过滤**（待实现）：
   - 在搜索框中输入关键字
   - 支持正则表达式
   - 支持多个关键字组合

### 控制日志流

- **暂停/恢复**：点击 Pause/Resume 按钮或按 `Ctrl+S`
- **清除日志**：点击 Clear 按钮或按 `Ctrl+L`
- **自动滚动**：新日志到达时自动滚动到底部（可在设置中关闭）

## 配置文件

配置文件位于 `~/.config/com.openclaw.ilogcat/config.toml`

```toml
[general]
default_log_level = "Info"
max_log_lines = 100000
auto_scroll = true

[ui]
font = "Monospace 12"
theme = "dark"

[colors]
verbose = "#808080"
debug = "#0066CC"
info = "#008800"
warn = "#FF8800"
error = "#CC0000"
fatal = "#CC0000"

[[ssh_servers]]
name = "My Server"
host = "192.168.1.100"
port = 22
username = "root"
auth = { type = "password", password = "your-password" }
```

## 常见问题

### Q: 应用无法启动
A: 确保已安装 GTK4 运行时库：
```bash
# Ubuntu/Debian
sudo apt install libgtk-4-1

# Fedora
sudo dnf install gtk4
```

### Q: 无法查看 journalctl 日志
A: 确保当前用户有权限访问 systemd 日志：
```bash
# 临时方案
sudo ilogcat

# 永久方案（推荐）
sudo usermod -aG systemd-journal $USER
# 重新登录后生效
```

### Q: SSH 连接失败
A: 检查以下几点：
1. 服务器地址和端口是否正确
2. 用户名和密码是否正确
3. 服务器是否允许密码登录（检查 `/etc/ssh/sshd_config` 中的 `PasswordAuthentication`）
4. 防火墙是否允许 SSH 连接

### Q: 日志显示乱码
A: 确保日志文件使用 UTF-8 编码。对于其他编码的日志文件，可以先转换：
```bash
iconv -f GBK -t UTF-8 input.log > output.log
```

## 技术栈

- **语言**: Rust
- **GUI 框架**: GTK4
- **SSH 库**: ssh2-rs
- **配置格式**: TOML

## 许可证

MIT License

## 反馈与贡献

欢迎提交 Issue 和 Pull Request！

项目地址：https://github.com/ayukyo/ilogcat
