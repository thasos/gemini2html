# par défaut : lance la 1ère recipe, sinon :
_default:
    just --list --unsorted

run:
    cargo run
# run test with cargo-insta
test:
    cargo-insta test --review
# review cargo-insta
review:
    cargo-insta review
build:
    cargo +nightly build --release -Z build-std=std --target x86_64-unknown-linux-gnu
clean:
    cargo clean

export PKG_CONFIG_SYSROOT_DIR := "/home/${USER}/.rustup/toolchains/nightly-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-musl"
# used with alpine OCI image
build_musl:
    RUSTFLAGS='-C target-feature=-crt-static' cargo +nightly build --release -Z build-std=std --target x86_64-unknown-linux-musl

# use flake.nix
nixshell shell='zsh':
    nix develop --command {{shell}}
