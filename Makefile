.PHONY: check fmt test lint changelog

CARGO := rustup run stable cargo

check: fmt test lint

fmt:
	$(CARGO) fmt --check

test:
	$(CARGO) test

lint:
	$(CARGO) clippy --all-targets -- -D warnings

changelog:
	git cliff -o CHANGELOG.md
