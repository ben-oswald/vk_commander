#!/bin/bash
set -e
cd "$(dirname "$0")" || exit
cd ..

VERSION=$(grep -m1 "^version" ./Cargo.toml | sed 's/version = "\([^"]*\)"/\1/')

INSTALLER_FILE="releases/windows/vk-commander-${VERSION}-x64-installer.exe"

if [ -f "$INSTALLER_FILE" ]; then
  echo "Build failed, version $VERSION already exists"
  exit 1
fi

echo "Installing Windows target if not already installed..."
rustup target add x86_64-pc-windows-gnu

echo "Building Rust project for Windows..."
cargo build --release --target x86_64-pc-windows-gnu

mkdir -p releases/windows

if command -v makensis >/dev/null 2>&1; then
  echo "Building Windows installer..."

  if [ ! -f "scripts/windows_installer.nsi" ]; then
    echo "Error: scripts/windows_installer.nsi file not found"
    exit 1
  fi

  makensis -DAPP_VERSION="${VERSION}" scripts/windows_installer.nsi

  if [ -f "releases/windows/vkCommanderInstaller.exe" ]; then
    mv "releases/windows/vkCommanderInstaller.exe" "$INSTALLER_FILE"
    echo "Successfully built: $INSTALLER_FILE"
  else
    echo "Error: Expected installer 'releases/windows/vkCommanderInstaller.exe' was not created"
    exit 1
  fi
else
  echo "NSIS not found, skipping installer creation"
  echo "To build Windows installer, install NSIS and ensure 'makensis' is in your PATH"
  exit 1
fi

echo "Windows build complete!"