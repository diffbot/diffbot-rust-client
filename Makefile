RUSTC ?= rustc
RUSTDOC ?= rustdoc
RUSTFLAGS ?= -L build/ -L ../rust-http/build/
RUSTLIBFLAGS ?= -O

diffbot_files=src/diffbot/lib.rs

all: build/.diffbot-lib

deps:
	cd ../rust-http && make

build/.diffbot-lib: $(diffbot_files)
	mkdir -p build/
	$(RUSTC) $(RUSTFLAGS) $(RUSTLIBFLAGS) src/diffbot/lib.rs --out-dir=build
	touch build/.diffbot-lib

build/tests: src/diffbot/test.rs build/.diffbot-lib
	$(RUSTC) $(RUSTFLAGS) --test -o build/tests src/diffbot/test.rs

docs: doc/diffbot/index.html

doc/diffbot/index.html: $(diffbot_files)
	$(RUSTDOC) $(RUSTFLAGS) src/diffbot/lib.rs
	cp other/diffy-d.png doc/diffbot/

check: all build/tests
	build/tests --test

clean:
	rm -rf build/

.PHONY: all deps docs clean check
