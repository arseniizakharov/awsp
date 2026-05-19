# Copy this file to github.com/nomadsre/homebrew-awsp as Formula/awsp-beta.rb.

class AwspBeta < Formula
  desc "Switch AWS SSO profiles across shell sessions"
  homepage "https://github.com/nomadsre/awsp"
  url "https://github.com/nomadsre/awsp/archive/refs/tags/v0.1.0-beta.2.tar.gz"
  sha256 "e3db6f23a3821321456dcfb17945c6f36eedaf6cb0971a1b9b2b76f1e3f1d36b"
  license any_of: ["MIT", "Apache-2.0"]
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
