class Ferrotunnel < Formula
  desc "Embeddable, extensible, and observable reverse tunnel for Rust developers"
  homepage "https://github.com/MitulShah1/ferrotunnel"
  url "https://github.com/MitulShah1/ferrotunnel/archive/refs/tags/v1.0.1.tar.gz"
  sha256 "" # Will be filled after release
  license "MIT OR Apache-2.0"
  head "https://github.com/MitulShah1/ferrotunnel.git", branch: "main"

  depends_on "rust" => :build
  depends_on "pkg-config" => :build

  def install
    system "cargo", "install", *std_cargo_args(path: "ferrotunnel-cli")
  end

  test do
    version_output = shell_output("#{bin}/ferrotunnel --version")
    assert_match "ferrotunnel", version_output

    # Test that server --help works
    help_output = shell_output("#{bin}/ferrotunnel server --help")
    assert_match "Run the tunnel server", help_output
  end
end
