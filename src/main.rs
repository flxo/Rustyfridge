#![feature(start, plugin, core_intrinsics)]
#![no_std]
#![plugin(macro_platformtree)]

extern crate zinc;

mod fridge;
mod filter;

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
            current = &adc0;
            led = &led;
            setpoint = &adc2;
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
    fridge::run(&p, 1000, None);
}

#[cfg(test)] #[macro_use] extern crate std;
#[cfg(test)] #[macro_use] extern crate time;
#[cfg(test)] #[macro_use] extern crate rand;
#[cfg(test)] mod test {

use zinc::drivers::chario::CharIO;
use zinc::hal::pin::{Adc, Gpio, GpioLevel, GpioDirection};
use zinc::hal::timer::Timer;

#[derive(Default)]
struct State {
    compressor: bool,
    led: bool,
    current: u32,
}

struct GpioCompressor<'a> {
    state: &'a ::std::cell::RefCell<State>,
}

impl<'a> GpioCompressor<'a> {
    fn new(s: &'a ::std::cell::RefCell<State>) -> GpioCompressor {
        GpioCompressor {
            state: s,
        }
    }
}

impl<'a> Gpio for GpioCompressor<'a> {
    fn set_high(&self) {
        self.state.borrow_mut().compressor = true;
    }

    fn set_low(&self) {
        self.state.borrow_mut().compressor = false;
    }

    fn level(&self) -> GpioLevel {
        match self.state.borrow_mut().compressor {
            false => GpioLevel::Low,
            true => GpioLevel::High,
        }
    }
    fn set_direction(&self, _: GpioDirection) {}
}

struct GpioLed<'a> {
    state: &'a ::std::cell::RefCell<State>,
}

impl<'a> GpioLed<'a> {
    fn new(s: &'a ::std::cell::RefCell<State>) -> GpioLed<'a> {
        GpioLed {
            state: s,
        }
    }
}

impl<'a> Gpio for GpioLed<'a> {
    fn set_high(&self) {
        self.state.borrow_mut().led = true;
    }

    fn set_low(&self) {
        self.state.borrow_mut().led = false;
    }

    fn level(&self) -> GpioLevel {
        match self.state.borrow().led {
            false => GpioLevel::Low,
            true => GpioLevel::High,
        }
    }
    fn set_direction(&self, _: GpioDirection) {}
}

#[test]
fn led() {
    let state = ::std::cell::RefCell::new(State::default());
    let c = GpioLed::new(&state);
    c.set_high();
    assert!(state.borrow().led);
    c.set_low();
    assert!(!state.borrow().led);
}

struct AdcCurrent<'a> {
    state: &'a ::std::cell::RefCell<State>,
}

impl<'a> AdcCurrent<'a> {
    fn new(s: &'a ::std::cell::RefCell<State>) -> AdcCurrent {
        AdcCurrent {
            state: s,
        }
    }
    fn clip(&self, value: i32, min: i32, max: i32) -> i32 {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }
}
impl<'a> Adc for AdcCurrent<'a> {
    fn read(&self) -> u32 {
        let a = &mut self.state.borrow_mut();

        let mut v = a.current as i32;
        let r = (::rand::random::<u8>() % 10) as i32;
        match a.compressor {
            false => v += r,
            true => v -= r,
        };

        a.current = self.clip(v, 0, 4096) as u32;
        a.current
    }
}

#[test]
fn adc_current() {
    let state = ::std::cell::RefCell::new(State::default());
    let c = GpioCompressor::new(&state);
    let a = AdcCurrent::new(&state);

    c.set_low();
    for _ in 0..100 {
        let a0 = a.read();
        let a1 = a.read();
        assert!(a0 <= a1);
    }

    c.set_high();
    for _ in 0..100 {
        let a0 = a.read();
        let a1 = a.read();
        assert!(a0 >= a1);
    }
}

#[derive(Default)]
struct AdcSetpoint;
impl Adc for AdcSetpoint {
    fn read(&self) -> u32 {
        0
    }
}

#[derive(Default)]
struct TestTimer;
impl Timer for TestTimer {
    fn get_counter(&self) -> u32 {
        (::time::precise_time_ns() / 1000) as u32
    }
}

#[test]
fn timer() {
    let t = TestTimer::default();
    let t1 = ::time::precise_time_ns();
    t.wait_ms(100);
    let t2 = ::time::precise_time_ns();
    assert!((t2 - t1) > 100000);
}

#[derive(Default)]
struct TestUart;
impl CharIO for TestUart {
    fn putc(&self, value: char) {
        print!("{}", value);
    }
}

#[test]
fn run() {
    let state = ::std::cell::RefCell::new(State::default());
    let compressor = GpioCompressor::new(&state);
    let led = GpioLed::new(&state);
    let current = AdcCurrent::new(&state);
    let setpoint = AdcSetpoint::default();
    let timer = TestTimer::default();
    let uart = TestUart::default();

    timer.wait_ms(500);

    let p = ::fridge::Platform {
        compressor: &compressor,
        led: &led,
        current: &current,
        setpoint: &setpoint,
        timer: &timer,
        uart: &uart,
    };
    ::fridge::run(&p, 100, Some(100));
}

}
