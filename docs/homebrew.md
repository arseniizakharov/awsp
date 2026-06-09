# Homebrew Publishing

`awsp` is distributed through a dedicated Homebrew tap repo:

- app repo: `github.com/arseniizakharov/awsp`
- tap repo: `github.com/arseniizakharov/homebrew-formulae`

Homebrew maps the tap name `arseniizakharov/formulae` to the repository
`arseniizakharov/homebrew-formulae`.

## Release

Tag the app repo:

```sh
git tag -a v1.0.1 -m "v1.0.1"
git push origin main --tags
```

Build and package the Apple Silicon macOS binary:

```sh
cargo build --release --target aarch64-apple-darwin
mkdir -p /tmp/awsp-v1.0.1-aarch64-apple-darwin
install -m 0755 target/aarch64-apple-darwin/release/awsp /tmp/awsp-v1.0.1-aarch64-apple-darwin/awsp
COPYFILE_DISABLE=1 tar -czf /tmp/awsp-v1.0.1-aarch64-apple-darwin.tar.gz -C /tmp awsp-v1.0.1-aarch64-apple-darwin
shasum -a 256 /tmp/awsp-v1.0.1-aarch64-apple-darwin.tar.gz
gh release upload v1.0.1 /tmp/awsp-v1.0.1-aarch64-apple-darwin.tar.gz --repo arseniizakharov/awsp
```

Copy `packaging/homebrew/awsp.rb` into the tap repo as:

```text
Formula/awsp.rb
```

The `v1.0.1` Apple Silicon binary checksum is:

```text
0a879acb0d157a4f1d01c4460268bb0bd81a462a7f73fced7912ff0e32494162
```

Install from another machine:

```sh
brew tap arseniizakharov/formulae
brew install awsp
```

## Local Formula Check

From the tap repo:

```sh
brew install ./Formula/awsp.rb
brew test awsp
```

The formula installs a prebuilt Apple Silicon binary on arm64 macOS. Other
platforms need their own release artifact before they are added to the formula.
