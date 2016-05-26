// Copyright (C) 2016 Felix Obenhuber
//
// Licensed under the Apache License, Version 2.0 (the "License");
// you may not use this file except in compliance with the License.
// You may obtain a copy of the License at
//
//      http://www.apache.org/licenses/LICENSE-2.0
//
// Unless required by applicable law or agreed to in writing, software
// distributed under the License is distributed on an "AS IS" BASIS,
// WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
// See the License for the specific language governing permissions and
// limitations under the License.

#![no_std]
#![feature(start, plugin, core_intrinsics)]
#![plugin(macro_platformtree)]

extern crate zinc;

use zinc::drivers::chario::CharIO;
use zinc::hal::pin::{Adc, Gpio};
use zinc::hal::timer::Timer;

const VERSION: &'static str = env!("CARGO_PKG_VERSION");
const TEMP_LOW: i32 = 5000;
const TEMP_MID: i32 = 10000;
const TEMP_HIGH: i32 = 15000;
const CONTROL_HYSTERESIS: i32 = 1000;
const ACTUAL_FILTER: i32 = 10;
const SETPOINT_FILTER: i32 = 5;

platformtree!(
    lpc17xx@mcu {
        clock {
            source = "main-oscillator";
            source_frequency = 12_000_000;
            pll {
                m = 50;
                n = 3;
                divisor = 4;
            }
        }

        timer {
            timer@1 {
                counter = 25;
                divisor = 4;
            }
        }

        uart {
            uart@0 {
                baud_rate = 115200;
                mode = "8N1";
                tx = &uart_tx;
                rx = &uart_rx;
            }
        }

        gpio {
            0 {
                uart_tx@2;
                uart_rx@3;
                compressor@8 { direction = "out"; }
                led@22 { direction = "out"; }
                adc0@23 { direction = "out"; function = "ad0_0"; }
                adc2@25 { direction = "out"; function = "ad0_2"; }
            }
        }
    }
os {
    single_task {
        loop = "run";
        args {
            compressor = &compressor;
            actual = &adc2;
            led = &led;
            setpoint = &adc0;
            timer = &timer;
            uart = &uart;
        }
    }
}
);

fn run(args: &pt::run_args) {
    args.uart.puts("Rustyfridge ");
    args.uart.puts(VERSION);
    args.uart.puts("\r\n");

    args.timer.wait_ms(2000);

    let mut loops: u32 = 0;
    let mut actual_filter: Option<i32> = None;
    let mut setpoint_filter: Option<i32> = None;
    let mut cool: bool = false;

    // Simple floating mean filter
    let filter = |value: i32, last: &mut Option<i32>, factor: i32| -> i32 {
        *last = match *last {
            Some(v) => Some(((v * (factor - 1)) + value) / factor),
            None => Some(value),
        };
        last.unwrap()
    };

    loop {
        // Actual value needs a fixed offset and factor for conversion to mdeg
        let actual = filter(args.actual.read() as i32, &mut actual_filter, ACTUAL_FILTER) * 100 - 4000;

        // Setpoint matches log poti in three ranges
        let setpoint = match filter(args.setpoint.read() as i32, &mut setpoint_filter, SETPOINT_FILTER) {
            0...180 => TEMP_LOW,
            181...660 => TEMP_MID,
            _ => TEMP_HIGH,
        };

        // Decide whether to cool or not
        cool = if (actual - setpoint).abs() > CONTROL_HYSTERESIS {
            actual > setpoint
        } else {
            cool
        };
        if cool {
            args.compressor.set_high();
        } else {
            args.compressor.set_low();
        }

        // Blink faster if compressor is running
        if loops & if cool {
            0x4
        } else {
            0x8
        } != 0 {
            args.led.set_high();
        } else {
            args.led.set_low();
        }

        // Basic formatting
        let print_deg = |value: i32| {
            if value > -100000 && value < 100000 {
                args.uart.puts(" ");
            }
            if value > -10000 && value < 10000 {
                args.uart.puts(" ");
            }
            let v = if value < 0 {
                args.uart.puts("-");
                -value as u32
            } else {
                args.uart.puts(" ");
                value as u32
            };
            args.uart.puti(v / 1000);
            args.uart.puts(".");
            args.uart.puti((v % 1000) / 100);
        };

        // Trace current state
        args.uart.puts(if cool {
            "❄ "
        } else {
            "  "
        });
        print_deg(actual);
        args.uart.puts(" | ");
        print_deg(setpoint);
        args.uart.puts(" | diff: ");
        print_deg(setpoint - actual);
        args.uart.puts("\r\n");

        // Update loop counter
        if loops == u32::max_value() {
            loops = 0;
        } else {
            loops += 1;
        }

        args.timer.wait_ms(100);
    }
}
