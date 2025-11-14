.PHONY: install
install:
	cargo build --release
	cp target/release/kla ~/.local/bin/kla

.PHONY: build
build: 
	cargo build

.PHONY: uninstall
uninstall:
	rm ~/.local/bin/kla
