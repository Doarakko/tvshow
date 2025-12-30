#!/bin/bash
set -e

if [ -z "$1" ]; then
  echo "Usage: $0 <version>"
  echo "Example: $0 0.1.3"
  exit 1
fi

VERSION=$1
REPO="Doarakko/tvshow"
TMP_DIR=$(mktemp -d)

echo "Downloading release assets for v${VERSION}..."
gh release download "v${VERSION}" --repo "$REPO" --pattern "*.tar.gz" --dir "$TMP_DIR"

echo ""
echo "Calculating sha256..."
X86_64_DARWIN=$(shasum -a 256 "$TMP_DIR/tvshow-x86_64-apple-darwin.tar.gz" | awk '{print $1}')
AARCH64_DARWIN=$(shasum -a 256 "$TMP_DIR/tvshow-aarch64-apple-darwin.tar.gz" | awk '{print $1}')
X86_64_LINUX=$(shasum -a 256 "$TMP_DIR/tvshow-x86_64-unknown-linux-gnu.tar.gz" | awk '{print $1}')

echo "x86_64-apple-darwin:      $X86_64_DARWIN"
echo "aarch64-apple-darwin:     $AARCH64_DARWIN"
echo "x86_64-unknown-linux-gnu: $X86_64_LINUX"

echo ""
echo "Generating formula..."

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
OUTPUT_FILE="$SCRIPT_DIR/../Formula/tvshow.rb"
mkdir -p "$(dirname "$OUTPUT_FILE")"

cat > "$OUTPUT_FILE" << EOF
class Tvshow < Formula
  desc "Display Japanese TV schedules in the terminal"
  homepage "https://github.com/Doarakko/tvshow"
  version "${VERSION}"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/Doarakko/tvshow/releases/download/v#{version}/tvshow-x86_64-apple-darwin.tar.gz"
      sha256 "${X86_64_DARWIN}"
    end

    on_arm do
      url "https://github.com/Doarakko/tvshow/releases/download/v#{version}/tvshow-aarch64-apple-darwin.tar.gz"
      sha256 "${AARCH64_DARWIN}"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/Doarakko/tvshow/releases/download/v#{version}/tvshow-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "${X86_64_LINUX}"
    end
  end

  def install
    bin.install "tvshow"
  end

  test do
    assert_match "tvshow", shell_output("#{bin}/tvshow --help")
  end
end
EOF

echo ""
echo "Generated: $OUTPUT_FILE"
cat "$OUTPUT_FILE"

echo ""
echo "Cleaning up..."
rm -rf "$TMP_DIR"

echo ""
echo "Done! Copy Formula/tvshow.rb to homebrew-tap/Formula/tvshow.rb"
