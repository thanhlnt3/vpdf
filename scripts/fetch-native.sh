#!/usr/bin/env bash
# Tải PDFium (bblanchon/pdfium-binaries) và qpdf dựng sẵn vào third_party/, kiểm tra SHA-256.
# Dùng: scripts/fetch-native.sh [win-x64|mac-arm64|mac-x64|mac-universal]   (mặc định: nền tảng hiện tại)
# Trên macOS, "mac-universal" tải cả hai kiến trúc rồi ghép libpdfium.dylib bằng lipo (task 1.3).
set -euo pipefail
cd "$(dirname "$0")/.."

PDFIUM_TAG="chromium/8138"
QPDF_VER="12.4.2"

# (macOS có bash 3.2, không có mảng kết hợp, nên dùng case.)
expected_sha() {
  case "$1" in
    pdfium-win-x64.tgz) echo 8d48973e3eb472dda0f5f2435e595ac043f9f2219dd71883392f549445afa8a5 ;;
    pdfium-mac-arm64.tgz) echo 21a1567b397b571f4f77f21ca91eac7b892774219c3c715a5127a20950ccb18b ;;
    pdfium-mac-x64.tgz) echo 72662ac8eff7bf5381ef82ceebd0a0ad5bddb051b303981115114e6972fd201c ;;
    qpdf-${QPDF_VER}-msvc64.zip) echo db87077e683630c1217e0e8f9a20a9749d952ab676e881c3689187763a5de25d ;;
    qpdf-${QPDF_VER}-bin-macos-arm64.zip) echo 62e46987a30ea167cbc530ccb22690aec3d8c812ed09979a941bcee92e504b79 ;;
    qpdf-${QPDF_VER}-bin-macos-x86_64.zip) echo dd3b01f4414d198529bb0f13bba59c2f016a328bb3506695087a96fb80fc1481 ;;
    *) echo unknown ;;
  esac
}

sha256() { if command -v sha256sum >/dev/null; then sha256sum "$1" | cut -d' ' -f1; else shasum -a 256 "$1" | cut -d' ' -f1; fi; }

fetch() { # url file
  local url="$1" file="$2" tmp
  tmp="$(mktemp -d)"
  curl -fsSL --retry 3 -o "$tmp/$file" "$url"
  local got; got="$(sha256 "$tmp/$file")"
  if [[ "$got" != "$(expected_sha "$file")" ]]; then
    echo "SHA-256 không khớp cho $file: $got" >&2; exit 1
  fi
  echo "$tmp/$file"
}

pdfium() { # plat
  local plat="$1" f; f="$(fetch "https://github.com/bblanchon/pdfium-binaries/releases/download/${PDFIUM_TAG}/pdfium-${plat}.tgz" "pdfium-${plat}.tgz")"
  mkdir -p "third_party/pdfium/${plat}"
  tar -xzf "$f" -C "third_party/pdfium/${plat}"
}

qpdf() { # plat asset
  local plat="$1" asset="$2" f; f="$(fetch "https://github.com/qpdf/qpdf/releases/download/v${QPDF_VER}/${asset}" "$asset")"
  mkdir -p "third_party/qpdf/${plat}"
  unzip -oq "$f" -d "third_party/qpdf/${plat}"
}

plat="${1:-}"
if [[ -z "$plat" ]]; then
  case "$(uname -s)-$(uname -m)" in
    Darwin-arm64) plat=mac-arm64 ;;
    Darwin-x86_64) plat=mac-x64 ;;
    MINGW*|MSYS*|CYGWIN*) plat=win-x64 ;;
    *) echo "nền tảng chưa hỗ trợ: $(uname -s)-$(uname -m)" >&2; exit 1 ;;
  esac
fi

case "$plat" in
  win-x64) pdfium win-x64; qpdf win-x64 "qpdf-${QPDF_VER}-msvc64.zip" ;;
  mac-arm64) pdfium mac-arm64; qpdf mac-arm64 "qpdf-${QPDF_VER}-bin-macos-arm64.zip" ;;
  mac-x64) pdfium mac-x64; qpdf mac-x64 "qpdf-${QPDF_VER}-bin-macos-x86_64.zip" ;;
  mac-universal)
    pdfium mac-arm64; pdfium mac-x64
    qpdf mac-arm64 "qpdf-${QPDF_VER}-bin-macos-arm64.zip"; qpdf mac-x64 "qpdf-${QPDF_VER}-bin-macos-x86_64.zip"
    mkdir -p third_party/pdfium/mac-universal/lib
    lipo -create third_party/pdfium/mac-arm64/lib/libpdfium.dylib third_party/pdfium/mac-x64/lib/libpdfium.dylib \
      -output third_party/pdfium/mac-universal/lib/libpdfium.dylib
    lipo -info third_party/pdfium/mac-universal/lib/libpdfium.dylib
    ;;
  *) echo "nền tảng lạ: $plat" >&2; exit 1 ;;
esac
echo "Đã tải thư viện native cho $plat."
