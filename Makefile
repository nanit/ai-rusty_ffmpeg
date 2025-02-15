

tag-and-push:
	cargo install cargo-release && \
	cargo release tag --workspace --execute --no-confirm --allow-branch master && \
	cargo release push --workspace --execute --no-confirm
