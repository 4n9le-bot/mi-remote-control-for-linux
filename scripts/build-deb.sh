#!/usr/bin/env bash

set -euo pipefail

repo_dir=$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)
if (( $# > 1 )); then
    printf 'usage: %s [OUTPUT_DIRECTORY]\n' "$0" >&2
    exit 2
fi
output_dir=${1:-"$repo_dir/target/debian"}
build_dir=$(mktemp -d)
trap 'rm -rf -- "$build_dir"' EXIT

command -v cargo >/dev/null
command -v dpkg >/dev/null
command -v dpkg-deb >/dev/null
command -v dpkg-shlibdeps >/dev/null
command -v jq >/dev/null

metadata=$(cargo metadata --manifest-path "$repo_dir/Cargo.toml" --no-deps --format-version 1)
package_name=$(jq -er '.packages[0].name' <<<"$metadata")
package_version=$(jq -er '.packages[0].version' <<<"$metadata")
target_dir=$(jq -er '.target_directory' <<<"$metadata")
architecture=$(dpkg --print-architecture)

cargo build --manifest-path "$repo_dir/Cargo.toml" --release --locked

package_root="$build_dir/debian/$package_name"
install -Dm755 "$target_dir/release/atvv-bridge" \
    "$package_root/usr/bin/atvv-bridge"
install -Dm755 "$target_dir/release/atvv-button-mapping-helper" \
    "$package_root/usr/libexec/atvv-bridge/atvv-button-mapping-helper"
install -Dm644 "$repo_dir/packaging/90-atvv-bridge.hwdb" \
    "$package_root/usr/lib/udev/hwdb.d/90-atvv-bridge.hwdb"
install -Dm644 "$repo_dir/packaging/io.github.atvv_bridge.desktop" \
    "$package_root/usr/share/applications/io.github.atvv_bridge.desktop"
install -Dm644 "$repo_dir/packaging/io.github.atvv_bridge-autostart.desktop" \
    "$package_root/etc/xdg/autostart/io.github.atvv_bridge.desktop"
install -Dm644 "$repo_dir/packaging/io.github.atvv_bridge.button-mapping.policy" \
    "$package_root/usr/share/polkit-1/actions/io.github.atvv_bridge.button-mapping.policy"
install -Dm644 "$repo_dir/README.md" \
    "$package_root/usr/share/doc/atvv-bridge/README.md"

mkdir -p "$build_dir/debian"
cat >"$build_dir/debian/control" <<EOF
Source: $package_name
Section: utils
Priority: optional
Maintainer: atvv-bridge contributors <4n9le-bot@users.noreply.github.com>
Standards-Version: 4.7.0

Package: $package_name
Architecture: $architecture
Description: ATVV Remote to Voxtype voice bridge
EOF

shlib_output=$(
    cd "$build_dir"
    dpkg-shlibdeps -O \
        -e"debian/$package_name/usr/bin/atvv-bridge" \
        -e"debian/$package_name/usr/libexec/atvv-bridge/atvv-button-mapping-helper"
)
dependencies="${shlib_output#shlibs:Depends=}, pkexec, udev"
installed_size=$(du -sk "$package_root" | cut -f1)

mkdir -p "$package_root/DEBIAN"
install -m755 "$repo_dir/packaging/postinst" "$package_root/DEBIAN/postinst"
install -m755 "$repo_dir/packaging/postrm" "$package_root/DEBIAN/postrm"
cat >"$package_root/DEBIAN/control" <<EOF
Package: $package_name
Version: $package_version
Section: utils
Priority: optional
Architecture: $architecture
Maintainer: atvv-bridge contributors <4n9le-bot@users.noreply.github.com>
Installed-Size: $installed_size
Depends: $dependencies
Homepage: https://github.com/4n9le-bot/mi-remote-control-for-linux
Description: ATVV Remote to Voxtype voice bridge
 Connects a paired ATVV Remote to Voxtype and Fcitx 5 from an unprivileged
 graphical desktop session.
EOF

mkdir -p -- "$output_dir"
package_path="$output_dir/${package_name}_${package_version}_${architecture}.deb"
dpkg-deb --build --root-owner-group "$package_root" "$package_path"
printf '%s\n' "$package_path"
