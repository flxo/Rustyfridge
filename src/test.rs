//
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

#[cfg(test)]
mod test {

use zinc::drivers::chario::CharIO;
use zinc::hal::pin::{Adc, Gpio, GpioLevel, GpioDirection};
use zinc::hal::timer::Timer;
use gnuplot::*;

struct State<'a> {
    gpios: ::std::vec::Vec<(&'a str, GpioLevel)>,
    current: u32,
    setpoint: u32,
}

impl<'a> State<'a> {
    fn new() -> State<'a> {
        State {
            gpios: ::std::vec::Vec::new(),
            current: 0,
            setpoint: 0,
        }
    }
}

impl<'a> State<'a> {
    fn gpio_state(&self, index: usize) -> GpioLevel {
        self.gpios[index].1
    }

    fn current_value(&self) -> u32 {
        self.current
    }

}

struct TestGpio<'a> {
    index: usize,
    state: &'a ::std::cell::RefCell<State<'a>>,
}

impl<'a> TestGpio<'a> {
    fn new(n: &'a str, s: &'a ::std::cell::RefCell<State<'a>>) -> TestGpio<'a> {
        s.borrow_mut().gpios.push((n, GpioLevel::Low));
        TestGpio {
            index: s.borrow_mut().gpios.len() - 1,
            state: s,
        }
    }

    fn set(&self, v: GpioLevel) {
        self.state.borrow_mut().gpios[self.index].1 = v;
    }
}

impl<'a> Gpio for TestGpio<'a> {
    fn set_high(&self) {
        (self as &TestGpio).set(GpioLevel::High);
    }

    fn set_low(&self) {
        (self as &TestGpio).set(GpioLevel::Low);
    }

    fn level(&self) -> GpioLevel {
        self.state.borrow_mut().gpio_state(self.index)
    }
    fn set_direction(&self, _: GpioDirection) {}
}

// Simulated temperature inside of fridge
struct AdcCurrent<'a> {
    state: &'a ::std::cell::RefCell<State<'a>>,
}

impl<'a> AdcCurrent<'a> {
    fn new(s: &'a ::std::cell::RefCell<State<'a>>) -> AdcCurrent {
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
        let r;
        match ::rand::random::<u8>() % 50  {
            0 => r = ::rand::random::<u32>() % 0xFFF,
            _ => {
                let mut v = a.current_value() as i32;
                let rnd = (::rand::random::<u8>() % 2) as i32;
                match a.gpio_state(0) {
                    GpioLevel::Low => v += rnd,
                    GpioLevel::High => v -= rnd,
                };

                r = self.clip(v, 0, 4096) as u32;
                a.current = r;
            }
        }
        r
    }
}

struct AdcSetpoint<'a> {
    state: &'a ::std::cell::RefCell<State<'a>>,
}

impl<'a> AdcSetpoint<'a> {
    fn new(s: &'a ::std::cell::RefCell<State<'a>>) -> AdcSetpoint {
        AdcSetpoint {
            state: s,
        }
    }
}

impl<'a> Adc for AdcSetpoint<'a> {
    fn read(&self) -> u32 {
        // The LPC1796 has a adc hardware bug. This random
        // offset tries to emulate this
        let a = &mut self.state.borrow_mut();
        a.setpoint = match ::rand::random::<u8>() % 100  {
            0 => ::rand::random::<u32>() % 0xFFF,
            _ => 0,
        };
        a.setpoint
    }
}

#[derive(Default)]
struct TestTimer;
impl Timer for TestTimer {
    fn get_counter(&self) -> u32 {
        (::time::precise_time_ns() / 1000) as u32
    }
}

#[derive(Default)]
struct TestUart;
impl CharIO for TestUart {
    fn putc(&self, value: char) {
        print!("{}", value);
    }
}

#[derive(Default)]
struct Logger {
    process_data: ::std::vec::Vec<::fridge::Data>,
}

impl ::fridge::Step for Logger {
    fn process(&mut self, data: &mut ::fridge::Data) {
        self.process_data.push(data.clone());
    }
}

impl Logger {
    fn plot(&self, file: &str) {
        let setpoint_adc_raw = self.process_data.iter().map(|s| s.setpoint_adc_raw).collect::<::std::vec::Vec<_>>();
        let current_adc_raw = self.process_data.iter().map(|s| s.current_adc_raw as f32 / 10.0).collect::<::std::vec::Vec<_>>();
        let setpoint_mdeg = self.process_data.iter().map(|s| s.setpoint_mdeg as f32 / 1000.0).collect::<::std::vec::Vec<_>>();
        let current_mdeg = self.process_data.iter().map(|s| s.current_mdeg as f32 / 1000.0).collect::<::std::vec::Vec<_>>();
        let setpoint_adc = self.process_data.iter().map(|s| s.setpoint_adc).collect::<::std::vec::Vec<_>>();
        let current_adc = self.process_data.iter().map(|s| s.current_adc as f32 / 10.0).collect::<::std::vec::Vec<_>>();
        let compressor = self.process_data.iter().map(|s| match s.compressor {
            false => 0.0,
            true => 1.0,
        }).collect::<::std::vec::Vec<_>>();

        let mut f = Figure::new();
        f.axes2d()
            .set_border(false, &[], &[LineWidth(0.5)])
            .set_x_label("", &[]).set_y_label("temp", &[])
            .set_y_range(AutoOption::Auto, AutoOption::Fix(15.0))
            .set_size(1.0, 1.0)
            .lines(0..current_mdeg.len(), current_mdeg, &[Caption("current [mdeg]"), Color("blue")])
            .lines(0..setpoint_mdeg.len(), setpoint_mdeg, &[Caption("setpoint [mdeg]"), Color("green")])
            .lines(0..current_adc.len(), current_adc, &[Caption("current"), Color("red")])
            .lines(0..setpoint_adc.len(), setpoint_adc, &[Caption("setpoint"), Color("cyan")])
            .lines(0..current_adc_raw.len(), current_adc_raw, &[Caption("current [raw]"), Color("red")])
            .lines(0..setpoint_adc_raw.len(), setpoint_adc_raw, &[Caption("setpoint [raw]"), Color("yellow")])
            .lines(0..compressor.len(), compressor, &[Caption("compressor"), Color("black")]);
        f.set_terminal("pdfcairo", file).show();
    }

}

#[test]
fn gpio() {
    let state = ::std::cell::RefCell::new(State::new());
    let c = TestGpio::new("test", &state);
    c.set_high();
    assert!(c.level() == GpioLevel::High);
    c.set_low();
    assert!(c.level() == GpioLevel::Low);
}

#[test]
fn timer() {
    let t = TestTimer::default();
    let t1 = ::time::precise_time_ns();
    t.wait_ms(100);
    let t2 = ::time::precise_time_ns();
    assert!((t2 - t1) > 100000);
}

#[test]
fn run() {
    let state = ::std::cell::RefCell::new(State::new());
    let compressor = TestGpio::new("compressor", &state);
    let led = TestGpio::new("led", &state);
    let current = AdcCurrent::new(&state);
    let setpoint = AdcSetpoint::new(&state);
    let timer = TestTimer::default();
    let uart = TestUart::default();

    let mut logger = Logger::default();

    timer.wait_ms(500);

    let p = ::fridge::Platform {
        compressor: &compressor,
        led: &led,
        current: &current,
        setpoint: &setpoint,
        timer: &timer,
        uart: &uart,
    };

    ::fridge::run(&p, &mut logger, Some(600));

    logger.plot("testrun.pdf");
}

}

