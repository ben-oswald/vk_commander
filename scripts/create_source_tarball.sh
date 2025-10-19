#!/bin/bash
set -e

VERSION="0.0.0"
NAME="vk_commander"
TARBALL="${NAME}-${VERSION}.tar.gz"

echo "Creating source tarball: $TARBALL"

cd ..

tar --exclude-vcs \
    --exclude='target' \
    --exclude='*.rpm' \
    --exclude='.git*' \
    --exclude='create-source-tarball.sh' \
    -czf "$TARBALL" "$NAME/"

mkdir -p ~/rpmbuild/{SOURCES,SPECS,BUILD,RPMS,SRPMS}

cp "$TARBALL" ~/rpmbuild/SOURCES/

echo "Tarball created and copied to ~/rpmbuild/SOURCES/"
echo "You can now run: rpmbuild -bb scripts/rpmbuild/vk_commander.spec"
