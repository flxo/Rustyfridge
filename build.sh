export TARGET=thumbv7m-none-eabi
export PLATFORM=lpc17xx

rustc --version
cargo build --target=$TARGET --features "mcu_$PLATFORM" --release
