.PHONY: install
install:
	cargo build --release
	sudo cp target/release/main /usr/local/bin/kla

.PHONY: build
build: 
	cargo build

.PHONY: uninstall
uninstall:
	sudo rm /usr/local/bin/kla
