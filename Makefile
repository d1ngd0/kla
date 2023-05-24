.PHONY: install
install:
	cargo build --release
	sudo cp target/release/kla /usr/local/bin/kla

.PHONY: build
build: 
	cargo build

.PHONY: uninstall
uninstall:
	sudo rm /usr/local/bin/kla
