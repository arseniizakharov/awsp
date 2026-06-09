# Homebrew Publishing

`awsp` is distributed through a dedicated Homebrew tap repo:

- app repo: `github.com/arseniizakharov/awsp`
- tap repo: `github.com/arseniizakharov/homebrew-awsp`

Homebrew maps the tap name `arseniizakharov/awsp` to the repository
`arseniizakharov/homebrew-awsp`.

## Release

Tag the app repo:

```sh
git tag -a v1.0.0 -m "v1.0.0"
git push origin main --tags
```

Build and package the Apple Silicon macOS binary:

```sh
cargo build --release --target aarch64-apple-darwin
mkdir -p /tmp/awsp-v1.0.0-aarch64-apple-darwin
install -m 0755 target/aarch64-apple-darwin/release/awsp /tmp/awsp-v1.0.0-aarch64-apple-darwin/awsp
COPYFILE_DISABLE=1 tar -czf /tmp/awsp-v1.0.0-aarch64-apple-darwin.tar.gz -C /tmp awsp-v1.0.0-aarch64-apple-darwin
shasum -a 256 /tmp/awsp-v1.0.0-aarch64-apple-darwin.tar.gz
gh release upload v1.0.0 /tmp/awsp-v1.0.0-aarch64-apple-darwin.tar.gz --repo arseniizakharov/awsp
```

Copy `packaging/homebrew/awsp.rb` into the tap repo as:

```text
Formula/awsp.rb
```

The `v1.0.0` Apple Silicon binary checksum is:

```text
f3d2090ccee044bf734164a067c221053f2aff41db534e6edfd590bcca5cc0d6
```

Install from another machine:

```sh
brew tap arseniizakharov/awsp
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
