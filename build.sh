export LD_LIBRARY_PATH=`pwd`/../rust/lib
export PATH=`pwd`/../rust/bin:$PATH
export TARGET=thumbv7m-none-eabi
export PLATFORM=lpc17xx

rustc --version
cargo build --target=$TARGET --features "mcu_$PLATFORM" && cp ./target/thumbv7m-none-eabi/debug/rustyfridge rustyfridge.axf
