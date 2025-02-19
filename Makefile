all: run 

.PHONY: all clean run push test

run: build rust-git
	./rust-git

build:
	cargo build --release && cp ./target/release/rust-git .

clean:
	rm -f rust-git 

test: rust-git 
	./wyag-tests.sh