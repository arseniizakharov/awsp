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

Compute the source archive checksum:

```sh
curl -L https://github.com/nomadsre/awsp/archive/refs/tags/v0.1.0-beta.2.tar.gz | shasum -a 256
```

Copy `packaging/homebrew/awsp-beta.rb` into the tap repo as:

```text
Formula/awsp-beta.rb
```

The `v0.1.0-beta.2` source archive checksum is already filled in:

```text
e3db6f23a3821321456dcfb17945c6f36eedaf6cb0971a1b9b2b76f1e3f1d36b
```

Install from another machine:

```sh
brew install nomadsre/awsp/awsp-beta
```

Install the latest `main` build:

```sh
brew install --HEAD nomadsre/awsp/awsp-beta
```

## Local Formula Check

From the tap repo:

```sh
brew install --build-from-source ./Formula/awsp-beta.rb
brew test awsp-beta
```

The beta formula builds from source. That is acceptable for early testing; bottled releases can come later.
