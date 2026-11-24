# `just jobs=N <recipe>` caps build and test parallelism for that run; unset leaves CARGO_BUILD_JOBS and
# NEXTEST_TEST_THREADS as inherited, or at cargo's default (the CPU count)
jobs := ""
export CARGO_BUILD_JOBS := if jobs != "" { jobs } else { env_var_or_default('CARGO_BUILD_JOBS', num_cpus()) }
export NEXTEST_TEST_THREADS := if jobs != "" { jobs } else { env_var_or_default('NEXTEST_TEST_THREADS', 'num-cpus') }

# The workspace version, the first `version = ` of Cargo.toml ([workspace.package])
version := `sed -n 's/^version = "\(.*\)"$/\1/p' Cargo.toml | head -1`

# List the recipes
default:
    @just --list

# The tests. `just test`: all of them. `just test --changed`: only the crates changed against main and their
# dependents (through changedtests when installed, otherwise the full suite)
test *flags:
    {{ if flags == "--changed" { "changedtests run" } else { just_executable() + " test-all" } }}

# Every test: nextest, the doctests (nextest does not run them), the Python package's with uv, the cards' script
test-all:
    cargo nextest run --workspace --no-fail-fast
    cargo test --workspace --doc -q
    {{ just_executable() }} test-py
    {{ just_executable() }} test-js

# The Python package's tests (vz_py/tests): uv builds the package with maturin into .venv first
test-py:
    uv run --group dev pytest -q

# The code-refs tests against the lowest markdown pyproject.toml declares (markdown>=3.7)
test-py-oldest-markdown:
    uv run --group dev --with markdown==3.7 pytest -q vz_py/tests/test_refs.py

# The cards' script on a fake page with node (vz_refs/tests/vz-refs.test.mjs); without node it says it skipped
test-js:
    if command -v node >/dev/null; then node --test vz_refs/tests/vz-refs.test.mjs; else echo "skipped: node missing, vz_refs/tests/vz-refs.test.mjs not run" >&2; fi

# Format, clippy, the docs, spl at the small-workspace tier
lint:
    cargo fmt --all --check
    cargo clippy --workspace --all-targets -- -D warnings
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps -q
    spl check

# The package as a book installs it: from git at HEAD, in a fresh uv cache; `vz --version` names the workspace
# version and `import vhrd_zensical` works in the same environment (first build about 1-2 minutes)
package-check:
    rm -rf target/package-check
    UV_CACHE_DIR=target/package-check uv run --no-project --with "vhrd-zensical @ git+file://{{ justfile_directory() }}@$(git rev-parse HEAD)" sh -c 'vz --version | grep -F "vz {{ version }} (" && python -c "import vhrd_zensical"'

# Build with the deploy profile and put vz on PATH (binkeep when installed, otherwise cargo install)
install:
    cargo build --profile deploy --locked -p vz_cli
    if command -v binkeep >/dev/null; then binkeep list put vhrd_zensical {{ justfile_directory() }}/deploy/binkeep.toml && binkeep install vz {{ justfile_directory() }}/target/deploy; else cargo install --locked --path vz_cli; fi

# Bring this PC up to main: refuse on local changes, pull, install, check that vz names this commit
deploy:
    test -z "$(git status --porcelain)" || { echo "local changes, commit or move them first:" >&2; git status --short >&2; exit 1; }
    git pull --ff-only
    {{ just_executable() }} install
    vz --version | grep -qF "($(git rev-parse --short HEAD)"
    vz --version
