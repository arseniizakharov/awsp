# Copy this file to github.com/arseniizakharov/homebrew-formulae as Formula/awsp.rb.

class Awsp < Formula
  desc "AWS CLI sessions manager"
  homepage "https://github.com/arseniizakharov/awsp"
  url "https://github.com/arseniizakharov/awsp/releases/download/v1.0.8/awsp-v1.0.8-aarch64-apple-darwin.tar.gz"
  sha256 "e05d181f7173421d085228dba0371be48e624dec5b1e71cbbcdb608e59d9ba78"
  license any_of: ["MIT", "Apache-2.0"]

  depends_on arch: :arm64
  depends_on :macos

  def install
    bin.install "awsp"
  end

  def caveats
    <<~EOS
      Homebrew installed awsp but did not modify your shell startup files.
      Homebrew also did not install AWS CLI; awsp checks for `aws` at runtime.

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
