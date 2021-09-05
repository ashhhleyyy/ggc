# GGC

A rust-based [Gemini](https://gemini.circumlunar.space/) server.

## Building
To compile the server, ensure that you have rust installed, and then simply run `cargo build --release` to compile in release mode (or `cargo build` to compile a debug build). The resulting binary will be created in `target/release/ggc(.exe)` (or `target/debug/ggc(.exe)` for debug builds).

## Configuration
Configuration is done by a `config.toml` in the directory that the server is launched from.
```toml
# Specify the port for the server to listen on.
listen_port = 1965

# Define a virtual site for example1.org
[sites."example1.org"]
# Path to the site's certificate file
server_certificate_file = "keys/server.cert"
# Path to the corresponding RSA key for the site
key_file = "keys/server.rsa"

# Specify that this sites source is a flat_dir (flat directory)
[sites."example1.org".flat_dir]
# Path that files should be loaded from
directory = "static/example-1"
# Will automatically generate an file-listing index for directories without an index.gmi. Defaults to false if not specified
auto_index = true
# Hide the version number displayed in the file listing pages. Defaults to false if not specified
hide_version = true

# Define a second virtual site for example2.org
[sites."example2.org"]
# Paths to a different certificate and key, useful if the certificate is only valid for one domain
server_certificate_file = "keys/server2.cert"
key_file = "keys/server2.rsa"

[sites."example2.org".flat_dir]
directory = "static/example-2"
auto_index = true
# Entirely disables the footer on file listings. Defaults to false
disable_footer = true
```

## What's the name stand for?
It's short for [Gemini Guidance Computer](https://en.wikipedia.org/wiki/Gemini_Guidance_Computer), which was the onboard computer used on the [Gemini missions](https://en.wikipedia.org/wiki/Project_Gemini).
