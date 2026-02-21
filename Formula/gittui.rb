class Gittui < Formula
  desc "A VS Code-like Git TUI tool built in Rust"
  homepage "https://github.com/maplefukku/gittui"
  version "0.1.0"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "https://github.com/maplefukku/gittui/releases/download/v#{version}/gittui-aarch64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    else
      url "https://github.com/maplefukku/gittui/releases/download/v#{version}/gittui-x86_64-apple-darwin.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "https://github.com/maplefukku/gittui/releases/download/v#{version}/gittui-aarch64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    else
      url "https://github.com/maplefukku/gittui/releases/download/v#{version}/gittui-x86_64-unknown-linux-gnu.tar.gz"
      sha256 "PLACEHOLDER"
    end
  end

  def install
    bin.install "gittui"
  end

  test do
    assert_match "gittui", shell_output("#{bin}/gittui --version 2>&1", 0)
  end
end
