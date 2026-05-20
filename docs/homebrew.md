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
mkdir -p /tmp/awsp-v0.1.0-beta.8-aarch64-apple-darwin
install -m 0755 target/aarch64-apple-darwin/release/awsp /tmp/awsp-v0.1.0-beta.8-aarch64-apple-darwin/awsp
tar -czf /tmp/awsp-v0.1.0-beta.8-aarch64-apple-darwin.tar.gz -C /tmp awsp-v0.1.0-beta.8-aarch64-apple-darwin
shasum -a 256 /tmp/awsp-v0.1.0-beta.8-aarch64-apple-darwin.tar.gz
gh release upload v0.1.0-beta.8 /tmp/awsp-v0.1.0-beta.8-aarch64-apple-darwin.tar.gz --repo nomadsre/awsp
```

Copy `packaging/homebrew/awsp-beta.rb` into the tap repo as:

```text
Formula/awsp-beta.rb
```

The `v0.1.0-beta.8` Apple Silicon binary checksum is already filled in:

```text
5e1a8260cbfefb384026f99ed5cd2800aae4b699cf1b837785fea22a835c347f
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

The beta formula installs a prebuilt Apple Silicon binary on arm64 macOS. Other platforms need their own release artifact before they are added to the formula.
