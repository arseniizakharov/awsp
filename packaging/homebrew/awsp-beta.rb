# Copy this file to github.com/nomadsre/homebrew-awsp as Formula/awsp-beta.rb.

class AwspBeta < Formula
  desc "Switch AWS SSO profiles across shell sessions"
  homepage "https://github.com/nomadsre/awsp"
  url "https://github.com/nomadsre/awsp/releases/download/v0.1.0-beta.6/awsp-v0.1.0-beta.6-aarch64-apple-darwin.tar.gz"
  sha256 "0985d0610ff6ede51a6362eea8a941f44c67da6a0a21f6c74c67fb3ab59fe7c5"
  license any_of: ["MIT", "Apache-2.0"]

  depends_on "awscli"
  depends_on "fzf"
  depends_on arch: :arm64
  depends_on :macos

  def install
    bin.install "awsp"
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
