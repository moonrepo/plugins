set windows-shell := ["pwsh.exe", "-Command"]

init:
	cargo install cargo-binstall
	cargo binstall cargo-insta cargo-nextest cargo-wasi cargo-release

build:
	moon run :build

check:
	moon run :check

lint:
	moon run :lint

test name="":
	just build
	cargo nextest run --workspace {{name}}
