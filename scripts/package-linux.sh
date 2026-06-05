#!/usr/bin/env bash
set -euo pipefail

VERSION="${VERSION:-0.1.0}"
DIST_DIR="${DIST_DIR:-dist}"
TARGET_DIR="${TARGET_DIR:-target/release}"
APP_NAME="glide"
ARCH="${ARCH:-x86_64}"

root_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
dist_dir="$root_dir/$DIST_DIR"
target_dir="$root_dir/$TARGET_DIR"
work_dir="$root_dir/target/package-linux"

gui_bin="$target_dir/glide-gui"
cli_bin="$target_dir/glide-cli"
server_bin="$target_dir/glide-server"
icon_src="$root_dir/crates/glide-gui/assets/128x128.png"

for path in "$gui_bin" "$cli_bin" "$server_bin" "$icon_src"; do
    if [[ ! -f "$path" ]]; then
        echo "Missing required file: $path" >&2
        exit 1
    fi
done

rm -rf "$work_dir"
mkdir -p "$dist_dir" "$work_dir"

desktop_file_content="[Desktop Entry]
Type=Application
Name=Glide
Comment=LAN-first clipboard and input sharing
Exec=glide-gui
Icon=glide
Categories=Utility;Network;
Terminal=false
"

deb_root="$work_dir/deb"
mkdir -p \
    "$deb_root/DEBIAN" \
    "$deb_root/usr/bin" \
    "$deb_root/usr/share/applications" \
    "$deb_root/usr/share/icons/hicolor/128x128/apps"

install -m 0755 "$gui_bin" "$deb_root/usr/bin/glide-gui"
install -m 0755 "$cli_bin" "$deb_root/usr/bin/glide-cli"
install -m 0755 "$server_bin" "$deb_root/usr/bin/glide-server"
install -m 0644 "$icon_src" "$deb_root/usr/share/icons/hicolor/128x128/apps/glide.png"
printf "%s" "$desktop_file_content" > "$deb_root/usr/share/applications/glide.desktop"

installed_size="$(du -sk "$deb_root/usr" | awk '{print $1}')"
cat > "$deb_root/DEBIAN/control" <<EOF
Package: glide
Version: $VERSION
Section: utils
Priority: optional
Architecture: amd64
Maintainer: Glide Maintainers <noreply@example.com>
Installed-Size: $installed_size
Depends: libc6, libgcc-s1, libfontconfig1, libfreetype6, libxkbcommon0, libxkbcommon-x11-0, libxcb1, libxcb-render0, libxcb-shape0, libxcb-xfixes0, libssl3 | libssl1.1
Description: LAN-first clipboard and input sharing
 Glide provides a lightweight Slint GUI, CLI and server for LAN-first
 clipboard synchronization and input sharing.
EOF

dpkg-deb --root-owner-group --build "$deb_root" "$dist_dir/glide_${VERSION}_amd64.deb"

appdir="$work_dir/Glide.AppDir"
mkdir -p \
    "$appdir/usr/bin" \
    "$appdir/usr/share/applications" \
    "$appdir/usr/share/icons/hicolor/128x128/apps"

install -m 0755 "$gui_bin" "$appdir/usr/bin/glide-gui"
install -m 0755 "$cli_bin" "$appdir/usr/bin/glide-cli"
install -m 0755 "$server_bin" "$appdir/usr/bin/glide-server"
install -m 0644 "$icon_src" "$appdir/usr/share/icons/hicolor/128x128/apps/glide.png"
printf "%s" "$desktop_file_content" > "$appdir/usr/share/applications/glide.desktop"
cp "$appdir/usr/share/applications/glide.desktop" "$appdir/glide.desktop"
cp "$icon_src" "$appdir/glide.png"

cat > "$appdir/AppRun" <<'EOF'
#!/usr/bin/env sh
set -eu
HERE="$(dirname "$(readlink -f "$0")")"
exec "$HERE/usr/bin/glide-gui" "$@"
EOF
chmod +x "$appdir/AppRun"

appimagetool_path="${APPIMAGETOOL:-}"
if [[ -z "$appimagetool_path" ]]; then
    if command -v appimagetool >/dev/null 2>&1; then
        appimagetool_path="$(command -v appimagetool)"
    else
        appimagetool_path="$work_dir/appimagetool-${ARCH}.AppImage"
        curl -L \
            "https://github.com/AppImage/AppImageKit/releases/download/continuous/appimagetool-${ARCH}.AppImage" \
            -o "$appimagetool_path"
        chmod +x "$appimagetool_path"
    fi
fi

APPIMAGE_EXTRACT_AND_RUN=1 ARCH="$ARCH" \
    "$appimagetool_path" "$appdir" "$dist_dir/glide-${VERSION}-${ARCH}.AppImage"

ls -lh "$dist_dir"
