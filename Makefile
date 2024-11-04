check:
	cargo test
	cargo clippy --fix --allow-dirty --allow-staged
