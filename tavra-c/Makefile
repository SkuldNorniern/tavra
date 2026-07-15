.PHONY: build test clean

CARGO ?= cargo
TARGET_DIR := ../target/debug
LIB := $(TARGET_DIR)/libtavra_c.a

build:
	$(CARGO) build -p tavra-c

test: build
	cc -Wall -Wextra -std=c11 -Iinclude examples/round_trip.c $(LIB) -lpthread -ldl -lm -o /tmp/tavra_round_trip
	/tmp/tavra_round_trip

clean:
	rm -f /tmp/tavra_round_trip
