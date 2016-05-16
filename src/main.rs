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
#![allow(dead_code)]

extern crate zinc;

mod fridge;
#[cfg(test)]
mod test;

#[cfg(test)]
#[macro_use]
extern crate std;
#[cfg(test)]
#[macro_use]
extern crate time;
#[cfg(test)]
#[macro_use]
extern crate rand;
#[cfg(test)]
#[macro_use]
extern crate gnuplot;

#[no_mangle]
#[cfg(feature = "mcu_lpc17xx")]
pub unsafe extern "C" fn __aeabi_memclr8(s: *mut u8, n: usize) -> *mut u8 {
    let mut i = 0;
    while i < n {
        *s.offset(i as isize) = 0u8;
        i += 1;
    }
    return s;
}

#[cfg(feature = "mcu_lpc17xx")]
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
            current = &adc2;
            led = &led;
            setpoint = &adc0;
            timer = &timer;
            uart = &uart;
        }
    }
}
);

#[cfg(feature = "mcu_lpc17xx")]
fn run(args: &pt::run_args) {
    let p = fridge::Platform {
        compressor: args.compressor,
        led: args.led,
        current: args.current,
        setpoint: args.setpoint,
        timer: args.timer,
        uart: args.uart,
    };
    let mut tracer = ::fridge::Trace::new(args.uart);
    fridge::run(&p, &mut tracer, None);
}
