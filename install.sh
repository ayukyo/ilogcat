#!/bin/bash
# iLogCat Installation Script

set -e

VERSION="0.2.0"
BINARY_NAME="ilogcat"
INSTALL_DIR="/usr/local/bin"
DESKTOP_FILE_DIR="/usr/share/applications"
ICON_DIR="/usr/share/icons/hicolor/256x256/apps"

echo "iLogCat Installer v${VERSION}"
echo "=============================="

# Check if running as root
if [ "$EUID" -ne 0 ]; then
    echo "Please run as root (use sudo)"
    exit 1
fi

# Detect distribution
if [ -f /etc/os-release ]; then
    . /etc/os-release
    DISTRO=$ID
else
    echo "Cannot detect distribution"
    exit 1
fi

echo "Detected distribution: $DISTRO"

# Install dependencies
echo "Installing dependencies..."
case $DISTRO in
    ubuntu|debian)
        apt-get update
        apt-get install -y libgtk-4-1 libssh2-1
        ;;
    fedora)
        dnf install -y gtk4 libssh2
        ;;
    arch)
        pacman -S --needed gtk4 libssh2
        ;;
    *)
        echo "Unsupported distribution: $DISTRO"
        echo "Please manually install: GTK4, libssh2"
        exit 1
        ;;
esac

# Check if binary exists
if [ ! -f "target/release/ilogcat" ] && [ ! -f "ilogcat" ]; then
    echo "Binary not found. Please build first with: cargo build --release"
    exit 1
fi

# Install binary
echo "Installing binary..."
if [ -f "target/release/ilogcat" ]; then
    cp target/release/ilogcat "$INSTALL_DIR/$BINARY_NAME"
else
    cp ilogcat "$INSTALL_DIR/$BINARY_NAME"
fi
chmod +x "$INSTALL_DIR/$BINARY_NAME"

# Create desktop entry
echo "Creating desktop entry..."
cat > "$DESKTOP_FILE_DIR/ilogcat.desktop" << 'EOF'
[Desktop Entry]
Name=iLogCat
Comment=Linux Log Viewer inspired by Android Studio Logcat
Exec=/usr/local/bin/ilogcat
Icon=ilogcat
Terminal=false
Type=Application
Categories=System;Monitoring;
Keywords=log;viewer;monitoring;
StartupNotify=true
EOF

# Create a simple icon (placeholder)
if [ ! -f "$ICON_DIR/ilogcat.png" ]; then
    echo "Creating icon placeholder..."
    # Create a simple colored square as placeholder
    # In production, use a proper icon
    convert -size 256x256 xc:#2196F3 -pointsize 30 -fill white -gravity center -annotate +0+0 "iLogCat" "$ICON_DIR/ilogcat.png" 2>/dev/null || \
    echo "Icon creation skipped (ImageMagick not installed)"
fi

# Update desktop database
echo "Updating desktop database..."
if command -v update-desktop-database &> /dev/null; then
    update-desktop-database "$DESKTOP_FILE_DIR"
fi

echo ""
echo "Installation complete!"
echo ""
echo "You can now run iLogCat by:"
echo "  1. Typing 'ilogcat' in terminal"
echo "  2. Searching 'iLogCat' in your application menu"
echo ""
echo "To uninstall, run: sudo rm $INSTALL_DIR/$BINARY_NAME $DESKTOP_FILE_DIR/ilogcat.desktop"
