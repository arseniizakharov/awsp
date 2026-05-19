# Copy this file to github.com/nomadsre/homebrew-awsp as Formula/awsp-beta.rb.

class AwspBeta < Formula
  desc "Switch AWS SSO profiles across shell sessions"
  homepage "https://github.com/nomadsre/awsp"
  url "https://github.com/nomadsre/awsp/archive/refs/tags/v0.1.0-beta.5.tar.gz"
  sha256 "751a9a3ff83f0693e949ff372c296f2f14d5329c5f20f9e3b469e49c18d54eb3"
  license any_of: ["MIT", "Apache-2.0"]
  head "https://github.com/nomadsre/awsp.git", branch: "main"

  depends_on "rust" => :build
  depends_on "awscli"
  depends_on "fzf"

  def install
    system "cargo", "install", *std_cargo_args(path: ".")
  end

  def caveats
    <<~EOS
      Homebrew installed awsp but did not modify your shell startup files.

      Enable shell integration once:
        awsp setup zsh

      For bash:
        awsp setup bash

      Then restart the shell, or enable it immediately:
        source "$HOME/.config/awsp/shell/awsp.sh"
    EOS
  end

  test do
    assert_match version.to_s, shell_output("#{bin}/awsp --version")
    assert_match "awsp shell integration", shell_output("#{bin}/awsp init zsh")
  end
end
