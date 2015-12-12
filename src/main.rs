#![feature(start, plugin, core_intrinsics)]
#![no_std]
#![plugin(macro_platformtree)]

extern crate zinc;
use zinc::drivers::chario::CharIO;
use zinc::hal::timer::Timer;

mod fridge;
use fridge::*;

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
            current = &adc0;
            led = &led;
            setpoint = &adc2;
            timer = &timer;
            uart = &uart;
        }
    }
}
);

fn run(args: &pt::run_args) {
    args.uart.puts("starting\n");

    let mut data = Data::default();
    let mut adc_filter: AdcFilter = AdcFilter::new(10, 10);
    let mut adc_input: AdcRead = AdcRead::new(args.current, args.setpoint);
    let mut compressor: Compressor = Compressor::new(args.compressor);
    let mut control: Control = Control::new(1500);
    let mut current: Current = Current::default();
    let mut setpoint: Setpoint = Setpoint::default();
    let mut state_led: StateLed = StateLed::new(args.led);

    loop {
        adc_input.process(&mut data);
        adc_filter.process(&mut data);
        setpoint.process(&mut data);
        current.process(&mut data);
        control.process(&mut data);
        compressor.process(&mut data);
        state_led.process(&mut data);
        args.timer.wait_ms(500);
    }
}
