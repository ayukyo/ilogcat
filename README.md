# iLogCat

A logcat-like log viewer for Linux, inspired by Android Studio Logcat.

## Features

- 🎨 **Color-coded log levels** - Different colors for VERBOSE, DEBUG, INFO, WARN, ERROR, FATAL
- 🔍 **Keyword filtering** - Filter by single or multiple keywords with regex support
- 📋 **Multiple log sources** - Execute commands, watch files, or connect via SSH
- 🔌 **SSH remote support** - View remote logs with the same filtering capabilities
- 💻 **Native Linux GUI** - Built with GTK4 for a native experience

## Installation

### Prerequisites (Ubuntu)

```bash
sudo apt install libgtk-4-dev libssh2-1-dev build-essential
```

### From Source

```bash
git clone https://github.com/{owner}/ilogcat.git
cd ilogcat/code
cargo build --release
./target/release/ilogcat
```

## Usage

```bash
# View local system logs
ilogcat --command "dmesg"

# Watch a log file
ilogcat --file /var/log/syslog

# Connect to SSH server
ilogcat --ssh production --command "journalctl -f"
```

## License

MIT