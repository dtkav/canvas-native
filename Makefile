TARGET ?= x86_64-unknown-linux-musl

.PHONY: build package

build:
	ionice -c2 -n7 nice -n 10 cargo build --release --locked --target $(TARGET)

package: build
	python3 scripts/package_release.py $(TARGET)
