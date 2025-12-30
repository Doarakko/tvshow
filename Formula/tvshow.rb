class Tvshow < Formula
  desc "Display Japanese TV schedules in the terminal"
  homepage "https://github.com/Doarakko/tvshow"
  version "0.1.2"
  license "MIT"

  on_macos do
    on_intel do
      url "https://github.com/Doarakko/tvshow/releases/download/v#{version}/tvshow-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end

    on_arm do
      url "https://github.com/Doarakko/tvshow/releases/download/v#{version}/tvshow-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    on_intel do
      url "https://github.com/Doarakko/tvshow/releases/download/v#{version}/tvshow-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install "tvshow"
  end

  test do
    assert_match "tvshow", shell_output("#{bin}/tvshow --help")
  end
end
