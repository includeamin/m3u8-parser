check:
	cargo fmt
	cargo test
	cargo clippy --fix --allow-dirty --allow-staged

changelog:
	git cliff -o CHANGELOG.md
