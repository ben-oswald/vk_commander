Name:           vk_commander
Version:        0.0.0
Release:        1%{?dist}
Summary:        vkCommander is an Desktop Manager for Valkey databases.

License:        AGPL-3.0
URL:            https://github.com/ben/vk_commander
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  wayland-devel
BuildRequires:  libxkbcommon-devel
BuildRequires:  desktop-file-utils

%global debug_package %{nil}

%undefine _disable_source_fetch

#Requires:

%description
<p>Valkey Insight is a graphical user interface for managing Valkey databases.</p>
<p>This is a <b>personal project</b> in a <b>very early development</b> stage and under <b>active</b> development.
  It still contains <b>a lot of bugs</b> and <b>missing features</b>. Use at your own risk and expect frequent changes.</p>

%prep
%setup -q -n vk_commander
rm -rf vendor .cargo/config.toml

%build
export CARGO_HOME=$(pwd)/.cargo
cargo build --release

%install
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_datadir}/applications
mkdir -p %{buildroot}%{_datadir}/pixmaps
mkdir -p %{buildroot}%{_datadir}/icons/hicolor/scalable/apps
mkdir -p %{buildroot}%{_datadir}/vk_commander/commands

install -m 755 target/release/vk_commander %{buildroot}%{_bindir}/
install -m 644 build_resources/misc/vk_commander.desktop %{buildroot}%{_datadir}/applications/
install -m 644 build_resources/app_icon/vk_commander.png %{buildroot}%{_datadir}/pixmaps/
install -m 644 build_resources/app_icon/vk_commander.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps
install -m 644 commands/*.json %{buildroot}%{_datadir}/vk_commander/commands/

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/vk_commander.desktop

%files
%license license.txt
%{_bindir}/vk_commander
%{_datadir}/applications/vk_commander.desktop
%{_datadir}/pixmaps/vk_commander.png
%{_datadir}/icons/hicolor/scalable/apps/vk_commander.svg
%{_datadir}/vk_commander/commands/*.json

%changelog
* Fri Oct 17 2025 ben <info@oswald.dev>
- Initial package version 0.0.0