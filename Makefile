EXTRA_FEATURES=
EXTRA_BUILD_OPTIONS=
LOG_FEATURE=",tracing"
TARGET_OPTION=

all: build build-logging build-tracing

check-style:
	cargo fmt --check --verbose

clippy:
	cargo clippy --all-features --no-deps

test:
	RUST_BACKTRACE=1 cargo test ${TARGET_OPTION} --no-default-features --features="env,fs,json,yaml,toml${LOG_FEATURE}" -- --nocapture

examples: example-basic

example-basic:
	APP_NAME__FOO__SERVER__ADDRESS="127.0.0.1" \
    APP_NAME__BAR__SQLITE__FILE="/path/to/app.db" \
    APP_NAME__BAZ__LOGGING__LEVEL="debug" \
    APP_NAME__QUX__HTTPS__INSECURE="false" \
    RUST_BACKTRACE=1 cargo run ${TARGET_OPTION} --no-default-features --features="env,fs,json,yaml,toml${LOG_FEATURE}" --example basic -- --trace 'env://?prefix=APP_NAME' 'fs:///tests/etc/?strip-slash=true'

docs:
	cargo doc --all-features

build: build-nothing build-default build-env build-fs build-json build-yaml build-toml build-qs remove-target
	@ echo ""
	cargo build ${TARGET_OPTION} ${EXTRA_BUILD_OPTIONS} --no-default-features --features="env,fs,json,yaml,toml,qs${EXTRA_FEATURES}"
	@ ls -sh target/*/**/libplugx_config*.rlib

remove-target:
	@ rm -rf target/*/*.rlib target/**/*.rlib target/**/**/*.rlib 2>/dev/null || true

build-nothing: remove-target
	@ echo ""
	cargo build ${TARGET_OPTION} ${EXTRA_BUILD_OPTIONS} --no-default-features
	@ ls -sh target/*/**/libplugx_config*.rlib

build-default: remove-target
	@ echo ""
	cargo build ${TARGET_OPTION} ${EXTRA_BUILD_OPTIONS} --no-default-features --features="default${EXTRA_FEATURES}"
	@ ls -sh target/*/**/libplugx_config*.rlib

build-env: remove-target
	@ echo ""
	cargo build ${TARGET_OPTION} ${EXTRA_BUILD_OPTIONS} --no-default-features --features="env${EXTRA_FEATURES}"
	@ ls -sh target/*/**/libplugx_config*.rlib

build-fs: remove-target
	@ echo ""
	cargo build ${TARGET_OPTION} ${EXTRA_BUILD_OPTIONS} --no-default-features --features="fs${EXTRA_FEATURES}"
	@ ls -sh target/*/**/libplugx_config*.rlib

build-json: remove-target
	@ echo ""
	cargo build ${TARGET_OPTION} ${EXTRA_BUILD_OPTIONS} --no-default-features --features="json${EXTRA_FEATURES}"
	@ ls -sh target/*/**/libplugx_config*.rlib

build-yaml: remove-target
	@ echo ""
	cargo build ${TARGET_OPTION} ${EXTRA_BUILD_OPTIONS} --no-default-features --features="yaml${EXTRA_FEATURES}"
	@ ls -sh target/*/**/libplugx_config*.rlib

build-toml: remove-target
	@ echo ""
	cargo build ${TARGET_OPTION} ${EXTRA_BUILD_OPTIONS} --no-default-features --features="toml${EXTRA_FEATURES}"
	@ ls -sh target/*/**/libplugx_config*.rlib

build-qs: remove-target
	@ echo ""
	cargo build ${TARGET_OPTION} ${EXTRA_BUILD_OPTIONS} --no-default-features --features="qs${EXTRA_FEATURES}"
	@ ls -sh target/*/**/libplugx_config*.rlib

build-logging: remove-target
	@ echo ""
	cargo build ${TARGET_OPTION} ${EXTRA_BUILD_OPTIONS} --no-default-features --features="logging"
	@ ls -sh target/*/**/libplugx_config*.rlib
	@ ${MAKE} build EXTRA_FEATURES=",logging" EXTRA_BUILD_OPTIONS="${EXTRA_BUILD_OPTIONS}"

build-tracing: remove-target
	@ echo ""
	cargo build ${TARGET_OPTION} ${EXTRA_BUILD_OPTIONS} --no-default-features --features="tracing"
	@ ls -sh target/*/**/libplugx_config*.rlib
	@ ${MAKE} build EXTRA_FEATURES=",tracing" EXTRA_BUILD_OPTIONS="${EXTRA_BUILD_OPTIONS}"
