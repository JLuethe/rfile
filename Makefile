CARGO ?= $(HOME)/.cargo/bin/cargo

.PHONY: build test verify benchmark reference database
build:
	$(CARGO) build --release --locked
test:
	$(CARGO) test --locked
verify:
	CARGO=$(CARGO) python3 tools/verify.py --benchmark
benchmark:
	CARGO=$(CARGO) python3 tools/benchmark.py
reference:
	bash tools/build_reference.sh
database: build
	python3 tools/rebuild_database.py --write
