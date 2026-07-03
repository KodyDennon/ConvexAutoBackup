#!/usr/bin/env sh
set -eu

VERSION="${CONVEX_AUTOBACKUP_VERSION:-v0.1.0-beta.1}"
REPO="${CONVEX_AUTOBACKUP_REPO:-KodyDennon/ConvexAutoBackup}"
INSTALL_ROOT="${CONVEX_AUTOBACKUP_INSTALL_ROOT:-$HOME/.local/convex-autobackup}"
BIN_DIR="${CONVEX_AUTOBACKUP_BIN_DIR:-$HOME/.local/bin}"
CONFIG_DIR="${CONVEX_AUTOBACKUP_CONFIG_DIR:-$HOME/.convex-autobackup}"
DATA_DIR="${CONVEX_AUTOBACKUP_DATA_DIR:-$HOME/.local/share/convex-autobackup}"
AUTOSTART=1
BIND="${CONVEX_AUTOBACKUP_BIND:-0.0.0.0:8976}"

for arg in "$@"; do
  case "$arg" in
    --no-autostart) AUTOSTART=0 ;;
    --help)
      echo "Usage: install.sh [--no-autostart]"
      exit 0
      ;;
    *) echo "Unknown argument: $arg" >&2; exit 1 ;;
  esac
done

os="$(uname -s | tr '[:upper:]' '[:lower:]')"
arch="$(uname -m)"
case "$os" in
  darwin) target_os="macos" ;;
  linux) target_os="linux" ;;
  *) echo "Unsupported OS: $os" >&2; exit 1 ;;
esac
case "$arch" in
  x86_64|amd64) target_arch="x86_64" ;;
  arm64|aarch64) target_arch="aarch64" ;;
  *) echo "Unsupported architecture: $arch" >&2; exit 1 ;;
esac

asset="convex-autobackup-${target_os}-${target_arch}.tar.gz"
base_url="https://github.com/${REPO}/releases/download/${VERSION}"
tmp_dir="$(mktemp -d)"
cleanup() { rm -rf "$tmp_dir"; }
trap cleanup EXIT

mkdir -p "$INSTALL_ROOT" "$BIN_DIR" "$CONFIG_DIR" "$DATA_DIR"
curl -fsSL "$base_url/$asset" -o "$tmp_dir/$asset"
curl -fsSL "$base_url/SHA256SUMS" -o "$tmp_dir/SHA256SUMS"

if command -v shasum >/dev/null 2>&1; then
  (cd "$tmp_dir" && grep " $asset\$" SHA256SUMS | shasum -a 256 -c -)
else
  (cd "$tmp_dir" && grep " $asset\$" SHA256SUMS | sha256sum -c -)
fi

tar -xzf "$tmp_dir/$asset" -C "$INSTALL_ROOT" --strip-components=1
for bin in convex-autobackup convex-autobackup-worker convex-autobackup-mcp; do
  if [ -f "$INSTALL_ROOT/$bin" ]; then
    ln -sf "$INSTALL_ROOT/$bin" "$BIN_DIR/$bin"
  fi
done

env_file="$CONFIG_DIR/env"
if [ ! -f "$env_file" ]; then
  if command -v openssl >/dev/null 2>&1; then
    master_key="$(openssl rand -base64 48)"
  else
    master_key="$(LC_ALL=C tr -dc 'A-Za-z0-9' </dev/urandom | head -c 64)"
  fi
  cat > "$env_file" <<EOF
CONVEX_AUTOBACKUP_DATA_DIR=$DATA_DIR
CONVEX_AUTOBACKUP_MASTER_KEY=$master_key
CONVEX_AUTOBACKUP_BIND=$BIND
RUST_LOG=info
EOF
  chmod 600 "$env_file"
  echo "Generated $env_file. Back up CONVEX_AUTOBACKUP_MASTER_KEY; losing it can make stored secrets unrecoverable."
fi

set -a
. "$env_file"
set +a

"$BIN_DIR/convex-autobackup" runner install --json

if [ "$AUTOSTART" -eq 1 ]; then
  if [ "$target_os" = "linux" ] && command -v systemctl >/dev/null 2>&1; then
    systemd_dir="$HOME/.config/systemd/user"
    mkdir -p "$systemd_dir"
    cat > "$systemd_dir/convex-autobackup.service" <<EOF
[Unit]
Description=ConvexAutoBackup
After=network-online.target

[Service]
EnvironmentFile=$env_file
ExecStart=$BIN_DIR/convex-autobackup supervise
Restart=always
RestartSec=5

[Install]
WantedBy=default.target
EOF
    systemctl --user daemon-reload
    systemctl --user enable --now convex-autobackup.service
  elif [ "$target_os" = "macos" ]; then
    plist="$HOME/Library/LaunchAgents/com.convexautobackup.service.plist"
    mkdir -p "$HOME/Library/LaunchAgents"
    cat > "$plist" <<EOF
<?xml version="1.0" encoding="UTF-8"?>
<!DOCTYPE plist PUBLIC "-//Apple//DTD PLIST 1.0//EN" "http://www.apple.com/DTDs/PropertyList-1.0.dtd">
<plist version="1.0">
<dict>
  <key>Label</key><string>com.convexautobackup.service</string>
  <key>ProgramArguments</key>
  <array>
    <string>$BIN_DIR/convex-autobackup</string>
    <string>supervise</string>
  </array>
  <key>EnvironmentVariables</key>
  <dict>
    <key>CONVEX_AUTOBACKUP_DATA_DIR</key><string>$CONVEX_AUTOBACKUP_DATA_DIR</string>
    <key>CONVEX_AUTOBACKUP_MASTER_KEY</key><string>$CONVEX_AUTOBACKUP_MASTER_KEY</string>
    <key>CONVEX_AUTOBACKUP_BIND</key><string>$CONVEX_AUTOBACKUP_BIND</string>
    <key>RUST_LOG</key><string>${RUST_LOG:-info}</string>
  </dict>
  <key>RunAtLoad</key><true/>
  <key>KeepAlive</key><true/>
  <key>StandardOutPath</key><string>$CONFIG_DIR/service.log</string>
  <key>StandardErrorPath</key><string>$CONFIG_DIR/service.err.log</string>
</dict>
</plist>
EOF
    launchctl unload "$plist" >/dev/null 2>&1 || true
    launchctl load "$plist"
  else
    echo "Autostart is not available on this host. Start manually with: $BIN_DIR/convex-autobackup supervise"
  fi
fi

if [ "$AUTOSTART" -eq 1 ]; then
  sleep 3
  "$BIN_DIR/convex-autobackup" doctor --json
else
  "$BIN_DIR/convex-autobackup" runner status --json
fi

echo "ConvexAutoBackup installed."
echo "URL: http://localhost:8976"
echo "Config: $env_file"
