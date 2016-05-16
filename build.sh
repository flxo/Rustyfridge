cargo build --target=thumbv7m-none-eabi --features "mcu_lpc17xx" --release --verbose
cp ./target/thumbv7m-none-eabi/release/rustyfridge rustyfridge.elf
