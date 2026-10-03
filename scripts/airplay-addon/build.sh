#!/usr/bin/env bash
# Builds the OmniHub AirPlay add-on: UxPlay (GPL-3.0, https://github.com/FDH2/UxPlay)
# compiled for Windows with MSYS2 UCRT64, plus the GStreamer runtime it needs.
#
# Run in an MSYS2 UCRT64 shell with the packages listed in
# .github/workflows/airplay-addon.yml installed:
#
#   scripts/airplay-addon/build.sh dist
#
# Produces dist/OmniHub-AirPlay-addon-x64.zip and dist/OmniHub-AirPlay-addon-sources.zip.
# OmniHub downloads the first one on demand; it is not part of the installer.

set -euo pipefail

UXPLAY_REPO=${UXPLAY_REPO:-https://github.com/FDH2/UxPlay}
# UxPlay 1.74 (development): its built-in mDNS responder removes the need for
# Apple's Bonjour service on Windows. Bump deliberately and re-test.
UXPLAY_COMMIT=${UXPLAY_COMMIT:-d8d99555473dc3fdfd3e23cdfd3d0ba3e7a8c209}

OUT=$(realpath -m "${1:-dist}")
PREFIX=/ucrt64
WORK=$(mktemp -d)
STAGE="$WORK/OmniHub-AirPlay"
trap 'rm -rf "$WORK"' EXIT

echo "== UxPlay $UXPLAY_COMMIT"
git clone --quiet --filter=blob:none "$UXPLAY_REPO" "$WORK/src"
git -C "$WORK/src" checkout --quiet "$UXPLAY_COMMIT"
cmake -S "$WORK/src" -B "$WORK/build" -G Ninja -DCMAKE_BUILD_TYPE=Release
cmake --build "$WORK/build"

mkdir -p "$STAGE/bin" "$STAGE/lib/gstreamer-1.0" "$STAGE/licenses"
cp "$WORK/build/uxplay.exe" "$STAGE/bin/"
# The plugin scanner sits next to the DLLs it needs.
cp "$PREFIX/libexec/gstreamer-1.0/gst-plugin-scanner.exe" "$STAGE/bin/"

# GStreamer plugins UxPlay's pipelines use (appsrc ! h264parse ! decodebin !
# videoconvert ! videoscale ! videoflip ! autovideosink, appsrc ! avdec_aac/alac
# ! audioconvert ! audioresample ! volume ! autoaudiosink, cover art and text).
PLUGINS=(
  coreelements app audioconvert audioresample volume playback typefindfunctions
  videoconvertscale pango autodetect videofilter imagefreeze jpeg level audioparsers directsound
  videoparsersbad d3d11 d3d12 wasapi wasapi2 libav
)
for p in "${PLUGINS[@]}"; do
  f="$PREFIX/lib/gstreamer-1.0/libgst$p.dll"
  [ -f "$f" ] || { echo "missing GStreamer plugin $p ($f)"; exit 1; }
  cp "$f" "$STAGE/lib/gstreamer-1.0/"
done

# Copy every UCRT64 DLL the executables and plugins load, recursively.
declare -A seen=()
queue=("$STAGE"/bin/*.exe "$STAGE"/lib/gstreamer-1.0/*.dll)
while [ ${#queue[@]} -gt 0 ]; do
  f=${queue[0]}
  queue=("${queue[@]:1}")
  while read -r dep; do
    name=$(basename "$dep")
    [ -n "${seen[$name]:-}" ] && continue
    seen[$name]=1
    cp "$dep" "$STAGE/bin/"
    queue+=("$STAGE/bin/$name")
  done < <(ldd "$f" | awk '{print $3}' | grep -i "^$PREFIX/bin/" || true)
done

# Licenses and where the source of every bundled package comes from.
{
  echo "OmniHub AirPlay add-on — third-party components"
  echo
  echo "UxPlay (GPL-3.0): $UXPLAY_REPO commit $UXPLAY_COMMIT"
  echo "  Its complete source is published next to this add-on as"
  echo "  OmniHub-AirPlay-addon-sources.zip, and at the URL above."
  echo
  echo "The other files are unmodified MSYS2 UCRT64 packages. Build recipes:"
  echo "https://github.com/msys2/MINGW-packages — source archives:"
  echo "https://repo.msys2.org/mingw/sources/"
  echo
  for f in "$STAGE"/bin/*.dll "$STAGE"/bin/gst-plugin-scanner.exe "$STAGE"/lib/gstreamer-1.0/*.dll; do
    rel=${f#"$STAGE"/}
    case "$rel" in
      bin/*) src="$PREFIX/bin/${rel#bin/}" ;;
      lib/gstreamer-1.0/*) src="$PREFIX/lib/gstreamer-1.0/${rel#lib/gstreamer-1.0/}" ;;
    esac
    [ "$rel" = bin/gst-plugin-scanner.exe ] && src="$PREFIX/libexec/gstreamer-1.0/gst-plugin-scanner.exe"
    pacman -Qqo "$src" 2> /dev/null || true
  done | sort -u | while read -r pkg; do
    pacman -Q "$pkg"
    short=${pkg#mingw-w64-ucrt-x86_64-}
    if [ -d "$PREFIX/share/licenses/$short" ]; then
      cp -r "$PREFIX/share/licenses/$short" "$STAGE/licenses/"
    fi
  done
} > "$STAGE/SOURCES.txt"
cp "$WORK/src/LICENSE" "$STAGE/licenses/UxPlay-LICENSE.txt"

version=$(sed -n 's/^#define VERSION "\(.*\)"/\1/p' "$WORK/src/uxplay.cpp")
gst=$(pacman -Q mingw-w64-ucrt-x86_64-gstreamer | awk '{print $2}')
cat > "$STAGE/addon.json" << EOF
{
  "name": "OmniHub AirPlay add-on",
  "uxplayVersion": "$version",
  "uxplayCommit": "$UXPLAY_COMMIT",
  "gstreamerVersion": "$gst",
  "built": "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
}
EOF

mkdir -p "$OUT"
rm -f "$OUT/OmniHub-AirPlay-addon-x64.zip" "$OUT/OmniHub-AirPlay-addon-sources.zip"
(cd "$WORK" && zip -q -r -9 "$OUT/OmniHub-AirPlay-addon-x64.zip" OmniHub-AirPlay)
git -C "$WORK/src" archive --format=zip --prefix="UxPlay-$UXPLAY_COMMIT/" -o "$OUT/OmniHub-AirPlay-addon-sources.zip" "$UXPLAY_COMMIT"

echo "== add-on contents"
du -sh "$STAGE" "$STAGE/bin" "$STAGE/lib/gstreamer-1.0"
ls -l "$OUT"
sha256sum "$OUT/OmniHub-AirPlay-addon-x64.zip"
