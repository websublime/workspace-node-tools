set windows-shell := ["powershell"]
set shell := ["bash", "-cu"]
alias ued := update-esbuild-diff
_default:
    just --list -u

setup:
    # Rust related setup
    cargo install
    # cargo install cargo-binstall
    # cargo binstall taplo-cli cargo-insta cargo-deny cargo-shear -y
    # Node.js related setup
    corepack enable
    pnpm install
    @echo "✅✅✅ Setup complete!"

roll:
    just roll-rust
    just roll-node
    # just roll-repo
    # just ued

roll-rust:
    just check-rust
    just test-rust
    just lint-rust

roll-node:
    just test-node
    just check-node
    just lint-node

# run tests for both Rust and Node.js
test: test-rust

test-rust:
    cargo test --workspace --exclude workspace_binding -- --test-threads=1 --nocapture

# Lint the codebase
lint: lint-rust

lint-rust:
    cargo fmt --all -- --check
    cargo clippy --workspace --all-targets -- --deny warnings
    cargo shear
