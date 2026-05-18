# Copy this file to github.com/nomadsre/homebrew-awsp as Formula/awsp-beta.rb.
# Replace REPLACE_WITH_SHA256 after tagging the app repository.

class AwspBeta < Formula
  desc "Switch AWS SSO profiles across shell sessions"
  homepage "https://github.com/nomadsre/awsp"
  url "https://github.com/nomadsre/awsp/archive/refs/tags/v0.1.0-beta.1.tar.gz"
  version "0.1.0-beta.1"
  sha256 "REPLACE_WITH_SHA256"
  license "MIT OR Apache-2.0"
  head "https://github.com/nomadsre/awsp.git", branch: "main"

  depends_on "rust" => :build
  depends_on "awscli"
  depends_on "fzf"

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/awsp --version")
    assert_match "awsp shell integration", shell_output("#{bin}/awsp init zsh")
  end
end
