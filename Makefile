all: run 

.PHONY: all clean run push test

run: build rgit
	./rgit

build:
	cargo build --release && cp ./target/release/rgit .

clean:
	rm -f rgit 

test: rgit
	./wyag-tests.sh