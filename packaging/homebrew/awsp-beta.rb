# Copy this file to github.com/nomadsre/homebrew-awsp as Formula/awsp-beta.rb.

class AwspBeta < Formula
  desc "Switch AWS SSO profiles across shell sessions"
  homepage "https://github.com/nomadsre/awsp"
  license any_of: ["MIT", "Apache-2.0"]
  version "0.1.0-beta.6"

  on_macos do
    on_arm do
      url "https://github.com/nomadsre/awsp/releases/download/v0.1.0-beta.6/awsp-v0.1.0-beta.6-aarch64-apple-darwin.tar.gz"
      sha256 "0985d0610ff6ede51a6362eea8a941f44c67da6a0a21f6c74c67fb3ab59fe7c5"
    end

    on_intel do
      url "https://github.com/nomadsre/awsp/archive/refs/tags/v0.1.0-beta.6.tar.gz"
      sha256 "51a14ca9477db27c87c8e5aa24e525763d28ce0b8f9765e708e0b2d279e93761"
      depends_on "rust" => :build
    end
  end

  on_linux do
    url "https://github.com/nomadsre/awsp/archive/refs/tags/v0.1.0-beta.6.tar.gz"
    sha256 "51a14ca9477db27c87c8e5aa24e525763d28ce0b8f9765e708e0b2d279e93761"
    depends_on "rust" => :build
  end

  depends_on "awscli"
  depends_on "fzf"

  def install
    if (buildpath/"Cargo.toml").exist?
      system "cargo", "install", *std_cargo_args(path: ".")
    else
      bin.install "awsp"
    end
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
