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
- 🌙 **Dark/Light theme** - Switch between themes (planned)
- 🌐 **Multi-language** - Chinese/English support (planned)
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

## Roadmap

### v0.3.0 (Planned)

- [ ] Improved tab titles showing log source names
- [ ] SSH connection status indicator
- [ ] SSH interactive command input
- [ ] Filter functionality fixes
- [ ] Dark/Light theme switcher
- [ ] Chinese/English language support

## License

MIT License - see [LICENSE](LICENSE) file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.

## Acknowledgments

- Inspired by Android Studio Logcat
- Built with [GTK4](https://gtk.org/) and [Rust](https://rust-lang.org/)
