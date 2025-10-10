#!/bin/bash
set -e
cd "$(dirname "$0")" || exit
cd ..

VERSION=$(grep -m1 "^version" ./Cargo.toml | sed 's/version = "\([^"]*\)"/\1/')

DEB_FILE="releases/debian/valkey-insight-${VERSION}-1-amd64.deb"
RPM_FILE="releases/fedora/valkey_insight-${VERSION}-1.x86_64.rpm"

if [ -f "$DEB_FILE" ] || [ -f "$RPM_FILE" ]; then
  echo "Build failed, version $VERSION already exists"
  exit 1
fi

echo "Building Rust project..."
cargo build --release

PKG_DIR="releases/debian/valkey-insight-${VERSION}-1-amd64"
mkdir -p "$PKG_DIR"

if [ ! -d "build_resources/debian" ]; then
  echo "Error: build_resources/debian directory not found"
  exit 1
fi

cp -r build_resources/debian/* "$PKG_DIR/"

mkdir -p "$PKG_DIR/usr/bin"
cp target/release/valkey_insight "$PKG_DIR/usr/bin/valkey-insight"
chmod 755 "$PKG_DIR/usr/bin/valkey-insight"

sed "s/^Version: .*/Version: ${VERSION}-1/" build_resources/debian/DEBIAN/control > "$PKG_DIR/DEBIAN/control"

echo "Building .deb package..."
dpkg-deb --build --root-owner-group "$PKG_DIR"

rm -rf "$PKG_DIR"

echo "Successfully built: $DEB_FILE"

echo "Building RPM package..."

if [ ! -f "scripts/valkey_insight.spec" ]; then
  echo "Error: scripts/valkey_insight.spec file not found in $(pwd)"
  echo "Please make sure the spec file is in the scripts directory"
  exit 1
fi

mkdir -p releases/fedora/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS}
mkdir -p releases/source

sed "s/^Version:.*/Version:        ${VERSION}/" scripts/valkey_insight.spec > releases/fedora/SPECS/valkey_insight.spec

tar -czf "releases/fedora/SOURCES/valkey_insight-${VERSION}.tar.gz" \
  --exclude='target' \
  --exclude='releases' \
  --exclude='.git' \
  --transform "s,^,valkey_insight/," \
  .

tar -czf "releases/source/valkey_insight-${VERSION}.tar.gz" \
  --exclude='target' \
  --exclude='releases' \
  --exclude='.git' \
  --transform "s,^,valkey_insight/," \
  .

rpmbuild --define "_topdir $(pwd)/releases/fedora" -bb releases/fedora/SPECS/valkey_insight.spec

rm -rf releases/fedora/{BUILD,BUILDROOT,SPECS,SOURCES,SRPMS}

echo "Successfully built: $RPM_FILE"