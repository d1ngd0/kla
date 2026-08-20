# List what we can do
default:
    @just --list

# Development helpers
mod dev

# Default install directory
install_dir := env_var_or_default("KLA_INSTALL_DIR", "${HOME}/.local/bin")

# Run the tests
test *args:
    cargo test {{args}}

# build and install kla
install: test
    cargo build --release
    cp target/release/kla "{{install_dir}}"

# build kla with cargo
build: test
    cargo build

# uninstall kla
uninstall:
    rm "{{install_dir}}"

# Make incremental commits when tests pass
savepoint:
    #!/bin/bash
    if (git branch --show-current | grep -oq "main"); then
        echo "󰊢 Main branch selected, not starting savepoint"
        exit 1
    fi
    savepoint --clear --filetype rs just test

release-bug:
    gh release create $(gh release list --json tagName | jq '.[].tagName' -r | sort --version-sort -r | head -n 1 | awk ' BEGIN { FS="." } ; { print $1 "." $2 "." $3+1}')

release-minor:
    gh release create $(gh release list --json tagName | jq '.[].tagName' -r | sort --version-sort -r | head -n 1 | awk ' BEGIN { FS="." } ; { print $1 "." $2+1 ".0"}')

release-major:
    gh release create $(gh release list --json tagName | jq '.[].tagName' -r | sort --version-sort -r | head -n 1 | awk ' BEGIN { FS="." } ; { print $1+1 ".0.0"}')
