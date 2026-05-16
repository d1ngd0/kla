# List what we can do
default:
    @just --list

# Development helpers
mod dev

# Default install directory
install_dir := env_var_or_default("KLA_INSTALL_DIR", "${HOME}/.local/bin")

# build and install kla
install:
    cargo build --release
    cp target/release/kla "{{install_dir}}"

# build kla with cargo
build:
    cargo build

# uninstall kla
uninstall:
    rm "{{install_dir}}"
