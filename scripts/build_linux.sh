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

if [ ! -f "build_resources/flatpak/dev.oswald.ValkeyInsight.yml" ]; then
  echo "Error: build_resources/flatpak/dev.oswald.ValkeyInsight.yml not found"
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
  flatpak-cargo-generator ./Cargo.lock -o build_resources/flatpak/generated-sources.json
elif [ -f "scripts/flatpak-cargo-generator.py" ]; then
  python3 scripts/flatpak-cargo-generator.py ./Cargo.lock -o build_resources/flatpak/generated-sources.json
else
  echo "Downloading flatpak-cargo-generator..."
  mkdir -p scripts
  curl -o scripts/flatpak-cargo-generator.py https://raw.githubusercontent.com/flatpak/flatpak-builder-tools/master/cargo/flatpak-cargo-generator.py
  chmod +x scripts/flatpak-cargo-generator.py
  python3 scripts/flatpak-cargo-generator.py ./Cargo.lock -o build_resources/flatpak/generated-sources.json
fi

echo "Creating source tarball for Flatpak (without vendored dependencies)..."
tar -czf "releases/source/valkey_insight-${VERSION}.tar.gz" \
  --exclude='target' \
  --exclude='releases' \
  --exclude='.git' \
  --exclude='vendor' \
  --transform "s,^,valkey_insight/," \
  .

echo "Copying flatpak files to release directory..."
cp build_resources/flatpak/dev.oswald.ValkeyInsight.yml releases/source/dev.oswald.ValkeyInsight.yml
cp build_resources/flatpak/generated-sources.json releases/source/generated-sources.json

ABSOLUTE_TARBALL_PATH=$(pwd)/releases/source/valkey_insight-${VERSION}.tar.gz
sed -i "s|path: releases/source/valkey_insight-VERSION\.tar\.gz|path: ${ABSOLUTE_TARBALL_PATH}|g" \
  releases/source/dev.oswald.ValkeyInsight.yml

echo "Building Flatpak package (using local sources)..."
flatpak-builder --repo=releases/flatpak/repo --force-clean releases/flatpak/build-dir build_resources/flatpak/dev.oswald.ValkeyInsight-local.yml
flatpak build-bundle releases/flatpak/repo "releases/flatpak/valkey-insight-${VERSION}.flatpak" dev.oswald.ValkeyInsight

echo "Flatpak manifest created: releases/source/dev.oswald.ValkeyInsight.yml"
echo "Flatpak cargo sources created: releases/source/generated-sources.json"
echo "Source tarball created: releases/source/valkey_insight-${VERSION}.tar.gz"