CFLAGS=-O2
TARGET_DIR=~/.cache/rust/x86_64-unknown-linux-gnu/debug

build/libbasic.so: build/basic.o
	gcc ${CFLAGS} -shared build/basic.o -o build/libbasic.so

build/basic.o: basic.c
	@mkdir -p build
	gcc ${CFLAGS} -c -Wall -fpic basic.c -o build/basic.o

build/standalone: standalone.c
	@mkdir -p build
	gcc ${CFLAGS} -Wall standalone.c -o build/standalone -lbevy_ecs -L ${TARGET_DIR}

.PHONY: standalone
standalone: build/standalone
	LD_LIBRARY_PATH=${TARGET_DIR} ./build/standalone

.PHONY: basic
basic: build/libbasic.so
	cd basic && cargo run

.PHONY: clean
clean:
	rm -f build/*
