# Homebrew Beta Publishing

`awsp` should be distributed through a dedicated Homebrew tap repo:

- app repo: `github.com/nomadsre/awsp`
- tap repo: `github.com/nomadsre/homebrew-awsp`

Homebrew maps the tap name `nomadsre/awsp` to the repository `nomadsre/homebrew-awsp`.

## First Beta

Tag a beta in the app repo:

```sh
git tag -a v0.1.0-beta.1 -m "v0.1.0-beta.1"
git push origin main --tags
```

Build and package the Apple Silicon macOS binary:

```sh
cargo build --release --target aarch64-apple-darwin
mkdir -p /tmp/awsp-v0.1.0-beta.6-aarch64-apple-darwin
install -m 0755 target/aarch64-apple-darwin/release/awsp /tmp/awsp-v0.1.0-beta.6-aarch64-apple-darwin/awsp
tar -czf /tmp/awsp-v0.1.0-beta.6-aarch64-apple-darwin.tar.gz -C /tmp awsp-v0.1.0-beta.6-aarch64-apple-darwin
shasum -a 256 /tmp/awsp-v0.1.0-beta.6-aarch64-apple-darwin.tar.gz
gh release upload v0.1.0-beta.6 /tmp/awsp-v0.1.0-beta.6-aarch64-apple-darwin.tar.gz --repo nomadsre/awsp
```

Copy `packaging/homebrew/awsp-beta.rb` into the tap repo as:

```text
Formula/awsp-beta.rb
```

The `v0.1.0-beta.6` Apple Silicon binary checksum is already filled in:

```text
0985d0610ff6ede51a6362eea8a941f44c67da6a0a21f6c74c67fb3ab59fe7c5
```

Install from another machine:

```sh
brew install nomadsre/awsp/awsp-beta
```

## Local Formula Check

From the tap repo:

```sh
brew install ./Formula/awsp-beta.rb
brew test awsp-beta
```

The beta formula installs a prebuilt Apple Silicon binary on arm64 macOS. Intel macOS and Linux currently fall back to a source build until release artifacts are published for those platforms.
