.PHONY: all
all: clean build docs-no-open check

.PHONY: clean
clean:
	@cargo clean
	@rm -f test_fifo_bounded_queue_c

build:
	@cargo build --release
	@cbindgen --config cbindgen.toml --crate buddy_memory_manager --output src/buddy_memory_manager.h --lang c
	@gcc src/tests/main.c -L./target/release -lfifo_bounded_buffer -o test_fifo_bounded_queue_c

check:
	@cargo test --no-fail-fast --release
	@LD_LIBRARY_PATH=./target/release ./test_fifo_bounded_queue_c

docs:
	@cargo -q doc --open

docs-no-open:
	@cargo -q doc

.PHONY: install-deps
install-deps:
	sudo apt-get update -y
	sudo apt-get install -y libio-socket-ssl-perl libmime-tools-perl
