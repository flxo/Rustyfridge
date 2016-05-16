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

use zinc::drivers::chario::CharIO;
use zinc::hal::pin::{Adc, Gpio};
use zinc::hal::timer::Timer;
use filter::filter::{Filter, MeanFilter, PlausibleFilter};

pub struct Clock<'a> {
    timer: &'a Timer,
}

impl<'a> Clock<'a> {
    fn new(t: &'a Timer) -> Clock {
        Clock {
            timer: t,
        }
    }

    fn now(&self) -> u64 {
        // unimplemented
        0
    }
}

#[derive(Default, Clone)]
pub struct Data {
    pub timestamp: u64,
    pub setpoint_adc_raw: i32,
    pub setpoint_adc: i32,
    pub setpoint_mdeg: i32,
    pub current_adc_raw: i32,
    pub current_adc: i32,
    pub current_mdeg: i32,
    pub compressor: bool,
}

pub trait Step {
    fn process(&mut self, data: &mut Data);
}

struct AdcRead<'s> {
    clock: &'s Clock<'s>,
    current: &'s Adc,
    setpoint: &'s Adc,
}

impl<'s> AdcRead<'s> {
    fn new(clk: &'s Clock<'s>, c: &'s Adc, s: &'s Adc) -> AdcRead<'s> {
        AdcRead {
            clock: clk,
            current: c,
            setpoint: s,
        }
    }

    fn clip(value: i32, min: i32, max: i32) -> i32 {
        if value < min {
            min
        } else if value > max {
            max
        } else {
            value
        }
    }
}

impl<'s> Step for AdcRead<'s> {
    fn process(&mut self, data: &mut Data) {
        data.timestamp = self.clock.now();
        let current = self.current.read() as i32;
        data.current_adc_raw = current;
        data.current_adc = AdcRead::clip(current, 0, 4096);
        let setpoint = self.setpoint.read() as i32;
        data.setpoint_adc_raw = setpoint;
        data.setpoint_adc = AdcRead::clip(setpoint, 0, 4096);
    }
}

#[derive(Default)]
pub struct Setpoint;

impl Setpoint {
    pub fn adc_to_mdeg(adc: i32) -> i32 {
        match adc {
            // The log poti in the fridge is very hard to adjust, so
            // I use three predefined temperature ranges
            0...180   => 5000,
            181...660 => 10000,
            _         => 15000,
        }
    }
}

impl Step for Setpoint {
    fn process(&mut self, data: &mut Data) {
        data.setpoint_mdeg = Setpoint::adc_to_mdeg(data.setpoint_adc)
    }
}

#[derive(Default)]
pub struct Current;

impl Current {
    pub fn adc_to_mdeg(adc: i32) -> i32 {
        // The temperature sensor fails by 4deg...
        adc * 100 - 4000
    }
}

impl Step for Current {
    fn process(&mut self, data: &mut Data) {
        data.current_mdeg = Current::adc_to_mdeg(data.current_adc);
    }
}

struct AdcFilter<'s> {
    clock: &'s Clock<'s>,
    current_filter_plausible: PlausibleFilter,
    setpoint_filter_plausible: PlausibleFilter,
    current_filter: MeanFilter,
    setpoint_filter: MeanFilter,
}

impl<'s> AdcFilter<'s> {
    fn new(clk: &'s Clock<'s>, setpoint: i32, current: i32) -> AdcFilter {
        AdcFilter {
            clock: clk,
            current_filter_plausible: PlausibleFilter::new(10, 500),
            setpoint_filter_plausible : PlausibleFilter::new(10, 500),
            current_filter: MeanFilter::new(current),
            setpoint_filter: MeanFilter::new(setpoint),
        }
    }
}

impl<'s> Step for AdcFilter<'s> {
    fn process(&mut self, data: &mut Data) {
        let _ = self.clock;
        // disable plausible filter
        //data.setpoint_adc = self.setpoint_filter_plausible.filter(data.setpoint_adc);
        data.setpoint_adc = self.setpoint_filter.filter(data.setpoint_adc);
        // disable plausible filter
        //data.current_adc = self.current_filter_plausible.filter(data.current_adc);
        data.current_adc = self.current_filter.filter(data.current_adc);
    }
}

struct StateLed<'s> {
    on: bool,
    pin: &'s Gpio,
}

impl<'s> StateLed<'s> {
    fn new(l: &'s Gpio) -> StateLed<'s> {
        StateLed {
            on: false,
            pin: l,
        }
    }
}

impl<'s> Step for StateLed<'s> {
    fn process(&mut self, _data: &mut Data) {
        self.on = match self.on {
            false => { self.pin.set_high(); true },
            true => { self.pin.set_low(); false },
        }
    }
}

struct Control {
    hysteresis_mdeg: i32,
}

impl Control {
    fn new(hysteresis: i32) -> Control {
        Control {
            hysteresis_mdeg: hysteresis,
        }
    }
}

impl Step for Control {
    fn process(&mut self, data: &mut Data) {
        if data.current_mdeg >= (data.setpoint_mdeg + self.hysteresis_mdeg) {
            data.compressor = true;
        } if data.current_mdeg <= (data.setpoint_mdeg - self.hysteresis_mdeg) {
            data.compressor = false;
        }
    }
}

struct Compressor<'s> {
    pin: &'s Gpio,
}

impl<'s> Compressor<'s> {
    fn new(p: &'s Gpio) -> Compressor {
        Compressor {
            pin: p,
        }
    }
}

impl<'s> Step for Compressor<'s> {
    fn process(&mut self, data: &mut Data) {
        match data.compressor {
            false => self.pin.set_low(),
            true  => self.pin.set_high(),
        }
    }
}

pub struct Trace<'s> {
    io: &'s CharIO,
}

impl<'s> Trace<'s> {
    pub fn new(cio: &'s CharIO) -> Trace {
        Trace {
            io: cio,
        }
    }
}

impl<'s> Step for Trace<'s> {
    fn process(&mut self, data: &mut Data) {
        
        // does not work
        // let p = |value| {
        //     let v;
        //     if value < 0 {
        //         self.io.puts("-");
        //         v = (value * -1) as u32
        //     } else {
        //         v = value as u32;
        //     }
        //     self.io.puti(v / 1000);
        //     self.io.puts(".");
        //     self.io.puti(v % 1000);
        //     self.io.puts(" deg");
        // };

        match data.compressor {
            true  => self.io.puts("[cooling]: "),
            false => self.io.puts("[stopped]: "),
        }
        self.io.puts("setpoint: ");
        self.io.puti(data.setpoint_mdeg as u32);
        self.io.puts("\t");
        self.io.puts("current: ");
        self.io.puti(data.current_mdeg as u32);
        self.io.puts("\r\n");
    }
}

pub struct Platform<'a> {
    pub compressor: &'a Gpio,
    pub led: &'a Gpio,
    pub current: &'a Adc,
    pub setpoint: &'a Adc,
    pub timer: &'a Timer,
    pub uart: &'a CharIO,
}

pub fn run(p: &Platform, logger: &mut Step, loops: Option<u32>) {
    let clock = Clock::new(p.timer);
    let mut data = Data::default();

    let mut adc_filter = AdcFilter::new(&clock, 100, 100);
    let mut adc_input = AdcRead::new(&clock, p.current, p.setpoint);
    let mut compressor = Compressor::new(p.compressor);
    let mut control = Control::new(1000);
    let mut current = Current::default();
    let mut setpoint = Setpoint::default();
    let mut state_led = StateLed::new(p.led);

    let mut r = || {
        adc_input.process(&mut data);
        adc_filter.process(&mut data);
        setpoint.process(&mut data);
        current.process(&mut data);
        control.process(&mut data);
        compressor.process(&mut data);
        state_led.process(&mut data);
        logger.process(&mut data);
    };

    match loops {
        Some(n) => for _ in 0..n { r(); }, // this is for testing
        None    => loop { r(); p.timer.wait_ms(100) },
    }
}
