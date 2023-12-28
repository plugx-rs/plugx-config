EXTRA_FEATURES=
EXTRA_BUILD_OPTIONS=
LOG_FEATURE=",tracing"

all: build build-logging build-tracing

check-style:
	cargo fmt --check --verbose

clippy:
	cargo clippy --all-features --no-deps

test:
	RUST_BACKTRACE=1 cargo test --no-default-features --features="env,fs,json,yaml,toml${LOG_FEATURE}" -- --nocapture

docs:
	cargo doc --all-features

build: remove-target build-nothing build-default build-env build-fs build-json build-yaml build-toml build-qs
	@ echo ""
	cargo build ${EXTRA_BUILD_OPTIONS} --no-default-features --features="env,fs,json,yaml,toml,qs${EXTRA_FEATURES}"
	@ ls -sh target/*/*.rlib

remove-target:
	@ rm -rf target/*/*.rlib 2>/dev/null || true

build-nothing:
	@ echo ""
	cargo build ${EXTRA_BUILD_OPTIONS} --no-default-features
	@ ls -sh target/*/*.rlib

build-default:
	@ echo ""
	cargo build ${EXTRA_BUILD_OPTIONS} --no-default-features --features="default${EXTRA_FEATURES}"
	@ ls -sh target/*/*.rlib

build-env:
	@ echo ""
	cargo build ${EXTRA_BUILD_OPTIONS} --no-default-features --features="env${EXTRA_FEATURES}"
	@ ls -sh target/*/*.rlib

build-fs:
	@ echo ""
	cargo build ${EXTRA_BUILD_OPTIONS} --no-default-features --features="fs${EXTRA_FEATURES}"
	@ ls -sh target/*/*.rlib

build-json:
	@ echo ""
	cargo build ${EXTRA_BUILD_OPTIONS} --no-default-features --features="json${EXTRA_FEATURES}"
	@ ls -sh target/*/*.rlib

build-yaml:
	@ echo ""
	cargo build ${EXTRA_BUILD_OPTIONS} --no-default-features --features="yaml${EXTRA_FEATURES}"
	@ ls -sh target/*/*.rlib

build-toml:
	@ echo ""
	cargo build ${EXTRA_BUILD_OPTIONS} --no-default-features --features="toml${EXTRA_FEATURES}"
	@ ls -sh target/*/*.rlib

build-qs:
	@ echo ""
	cargo build ${EXTRA_BUILD_OPTIONS} --no-default-features --features="qs${EXTRA_FEATURES}"
	@ ls -sh target/*/*.rlib

build-logging:
	@ echo ""
	cargo build ${EXTRA_BUILD_OPTIONS} --no-default-features --features="logging"
	@ ls -sh target/*/*.rlib
	@ ${MAKE} build EXTRA_FEATURES=",logging" EXTRA_BUILD_OPTIONS="${EXTRA_BUILD_OPTIONS}"

build-tracing:
	@ echo ""
	cargo build ${EXTRA_BUILD_OPTIONS} --no-default-features --features="tracing"
	@ ls -sh target/*/*.rlib
	@ ${MAKE} build EXTRA_FEATURES=",tracing" EXTRA_BUILD_OPTIONS="${EXTRA_BUILD_OPTIONS}"
