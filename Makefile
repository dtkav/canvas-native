TARGET ?= x86_64-unknown-linux-musl
DIST_DIR ?= dist
VERSION := $(shell sed -n 's/^version = "\([^"]*\)"/\1/p' Cargo.toml)
ARCHIVE := canvas-native-v$(VERSION)-$(TARGET).tar.gz

.PHONY: build package

build:
	ionice -c2 -n7 nice -n 10 cargo build --release --locked --target $(TARGET)

package: build
	mkdir -p $(DIST_DIR)
	tar -czf $(DIST_DIR)/$(ARCHIVE) -C target/$(TARGET)/release canvas-native -C $(CURDIR) LICENSE -C $(CURDIR)/assets Inter-LICENSE.txt
	cd $(DIST_DIR) && sha256sum $(ARCHIVE) > $(ARCHIVE).sha256
