= Isar82 

This project implements a simple control loop for a mobile fridge.
The orignal electronics of a Waeco CoolFreeze 25 did not perform
well, so it is replaced with a LPCxpresso board that runs this
Zinc application. The hardware is quite simple:

* ADC channel to read the temperature inside the fridge
* ADC channel to read the setpoint controlled by the user
* GPIO to control the compressor

This software is for learning Rust and play with Zinc on Cortex M.
Things may not be optimal... ;-)

== Build

Zinc is quite picky about the used Rust toolchain. One of the latest usable versions
is the Nightly from January 2016. Install via Multirust:

```
multirust override nightly-2016-01-12
```

Target build:
```
cargo build --target=thumbv7m-none-eabi --features "mcu_lpc17xx" --release --verbose
```

Build and run tests:
```
cargo test --features test
```
