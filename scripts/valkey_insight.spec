Name:           valkey_insight
Version:        0.0.0
Release:        1%{?dist}
Summary:        A Valkey insight application built with Rust and egui

License:        AGPL-3.0
URL:            https://github.com/ben/valkey_insight
Source0:        %{name}-%{version}.tar.gz

BuildRequires:  rust
BuildRequires:  cargo
BuildRequires:  gcc
BuildRequires:  wayland-devel
BuildRequires:  libxkbcommon-devel
BuildRequires:  desktop-file-utils
#Requires:

%description
A Valkey insight application built with Rust and the egui GUI framework.

%prep
%setup -q -n valkey_insight

%build
cargo build --release

%install
mkdir -p %{buildroot}%{_bindir}
mkdir -p %{buildroot}%{_datadir}/applications
mkdir -p %{buildroot}%{_datadir}/pixmaps
mkdir -p %{buildroot}%{_datadir}/icons/hicolor/scalable/apps
mkdir -p %{buildroot}%{_datadir}/valkey_insight/commands

install -m 755 target/release/valkey_insight %{buildroot}%{_bindir}/
install -m 644 build_resources/misc/valkey_insight.desktop %{buildroot}%{_datadir}/applications/
install -m 644 build_resources/app_icon/valkey_insight.png %{buildroot}%{_datadir}/pixmaps/
install -m 644 build_resources/app_icon/valkey_insight.svg %{buildroot}%{_datadir}/icons/hicolor/scalable/apps
install -m 644 commands/*.json %{buildroot}%{_datadir}/valkey_insight/commands/

%check
desktop-file-validate %{buildroot}%{_datadir}/applications/valkey_insight.desktop

%files
%license license.txt
%{_bindir}/valkey_insight
%{_datadir}/applications/valkey_insight.desktop
%{_datadir}/pixmaps/valkey_insight.png
%{_datadir}/icons/hicolor/scalable/apps/valkey_insight.svg
%{_datadir}/valkey_insight/commands/*.json

%changelog
* Fr October 10 2025 ben <info@oswald.dev>
- Initial package version 0.0.0