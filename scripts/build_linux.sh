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

if [ ! -d "build_resources/debian" ]; then
  echo "Error: build_resources/debian directory not found"
  exit 1
fi

if [ ! -f "scripts/valkey_insight.spec" ]; then
  echo "Error: scripts/valkey_insight.spec file not found"
  exit 1
fi

mkdir -p releases/{debian,fedora/{BUILD,BUILDROOT,RPMS,SOURCES,SPECS,SRPMS},source,flatpak}

# ===== SINGLE BUILD =====
echo "Building Rust project..."
cargo build --release

# ===== VENDOR DEPENDENCIES (without rebuilding) =====
echo "Vendoring Cargo dependencies..."
cargo vendor vendor > .cargo-vendor-config.toml

# ===== DEB PACKAGE =====
echo "Building .deb package..."
PKG_DIR="releases/debian/valkey-insight-${VERSION}-1-amd64"
mkdir -p "$PKG_DIR/usr/bin"

cp -r build_resources/debian/* "$PKG_DIR/"
cp target/release/valkey_insight "$PKG_DIR/usr/bin/valkey-insight"
chmod 755 "$PKG_DIR/usr/bin/valkey-insight"

sed "s/^Version: .*/Version: ${VERSION}-1/" build_resources/debian/DEBIAN/control > "$PKG_DIR/DEBIAN/control"

dpkg-deb --build --root-owner-group "$PKG_DIR"
rm -rf "$PKG_DIR"
echo "Successfully built: $DEB_FILE"

# ===== VENDOR DEPENDENCIES FOR RPM =====
echo "Vendoring Cargo dependencies..."
cargo vendor vendor > .cargo-vendor-config.toml

mkdir -p .cargo
cat > .cargo/config.toml << 'EOF'
[source.crates-io]
replace-with = "vendored-sources"

[source.vendored-sources]
directory = "vendor"
EOF

# ===== RPM PACKAGE =====
echo "Building RPM package..."

sed "s/^Version:.*/Version:        ${VERSION}/" scripts/valkey_insight.spec > releases/fedora/SPECS/valkey_insight.spec

echo "Creating source tarball for RPM (with vendored dependencies)..."
tar -czf "releases/fedora/SOURCES/valkey_insight-${VERSION}.tar.gz" \
  --exclude='target' \
  --exclude='releases' \
  --exclude='.git' \
  --dereference \
  --transform "s,^,valkey_insight/," \
  .

rpmbuild --define "_topdir $(pwd)/releases/fedora" -bb releases/fedora/SPECS/valkey_insight.spec
rm -rf releases/fedora/{BUILD,BUILDROOT,SPECS,SOURCES,SRPMS}
echo "Successfully built: $RPM_FILE"

# ===== FLATPAK PACKAGE =====
echo "Cleaning vendored dependencies for Flatpak..."
rm -rf vendor .cargo .cargo-vendor-config.toml

echo "Generating Flatpak cargo dependencies from Cargo.lock..."
if command -v flatpak-cargo-generator &> /dev/null; then
  flatpak-cargo-generator ./Cargo.lock -o generated-sources.json
elif [ -f "scripts/flatpak-cargo-generator.py" ]; then
  python3 scripts/flatpak-cargo-generator.py ./Cargo.lock -o generated-sources.json
else
  echo "Downloading flatpak-cargo-generator..."
  mkdir -p scripts
  curl -o scripts/flatpak-cargo-generator.py https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
  chmod +x scripts/flatpak-cargo-generator.py
  python3 scripts/flatpak-cargo-generator.py ./Cargo.lock -ogenerated-sources.json
fi