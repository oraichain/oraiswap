gents: 
	cwtools gents contracts/*
gents-test:
	cwtools gents contracts/* -o cwsimulate/build/contracts
build: gents
	cwtools build contracts/* -o build/wasm
build-test: gents-test
	cwtools build contracts/* -o cwsimulate/build/wasm
clean: 
	rm -rf build && rm -rf cwsimulate/build && rm -rf contracts/*/artifacts

.PHONY: all clean build build-test gents gents-test
