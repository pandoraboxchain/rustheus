#!/bin/bash

# TODO db test are wierdly broken
# TODO verification tests need block builder to function properly
cargo test\
	-p chain\
	-p bitcrypto\
	-p serialization\
	-p message\
	-p params\
	-p primitives\
	-p keys\
	-p script\
	-p verification\
	-p memory_pool\
