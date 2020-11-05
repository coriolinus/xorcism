# `xorcism`: kind of like encryption

It's not encryption, of course; it's just barely more than `rot13`. Still, it symmetrically obfuscates a message with a key, which is something.

This implementation is written in terms of the `Write` and `Read` traits.

## Features

This package has a single feature, `bin`, which is enabled by default.
When enabled, `bin` produces a binary `xorcism` which enables command-line usage of this package.

To disable production of the binary and just use the library, include this as

```toml
xorcism = { version = "*", default-features = false }
```
