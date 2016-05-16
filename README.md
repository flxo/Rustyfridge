Rustyfridge
============

This project implements a simple control loop for a mobile fridge.
The orignal electronics of a Waeco CoolFreeze 25 did not work at
all (freezing beer and warm milk), so it is replaced with a LPCxpresso
board that runs this Zinc application. 

The hardware is quite simple:

* ADC channel to read the temperature inside the fridge
* ADC channel to read the setpoint controlled by the user
* GPIO to control the compressor

This software is for learning Rust and play with Zinc on Cortex M.
Things may not be optimal... ;-)

Build
-----

Target build:

```
cargo build --target=thumbv7m-none-eabi --features "mcu_lpc17xx" --release --verbose
```

Build and run tests:

```
cargo test --features test
```
