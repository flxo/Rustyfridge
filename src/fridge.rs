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

/// Floating mean filter
pub struct MeanFilter {
    last: Option<f32>,
    num: f32,
}

impl MeanFilter {
    pub fn new(n: i32) -> MeanFilter {
        MeanFilter {
            last: None,
            num: n as f32,
        }
    }

    fn filter(&mut self, value: i32) -> i32 {
        self.last = match self.last {
            Some(l) => Some((l * (self.num - 1.0) + value as f32) / self.num),
            None => Some(value as f32),
        };
        self.last.unwrap() as i32
    }
}

/// Very special filter for reading but LPC17xx adc
/// that tends to output invalid values
pub struct PlausibleFilter {
    num_fails: u32,
    fails: u32,
    diff: i32,
    last: Option<i32>,
}

impl PlausibleFilter {
    pub fn new(n: u32, d: i32) -> PlausibleFilter {
        PlausibleFilter {
            num_fails: n,
            fails: 0,
            diff: d,
            last: None,
        }
    }

    fn filter(&mut self, value: i32) -> i32 {
        match self.last {
            Some(x) => {
                if (x - value).abs() <= self.diff {
                    self.fails = 0;
                    self.last = Some(value);
                    value
                } else {
                    self.fails += 1;
                    if self.fails > self.num_fails {
                        self.fails = 0;
                        self.last = Some(value);
                        value
                    } else {
                        self.last.unwrap()
                    }
                }
            }
            None => {
                self.last = Some(value);
                value
            }
        }
    }
}

struct AdcRead<'s> {
    current: &'s Adc,
    setpoint: &'s Adc,
}

impl<'s> AdcRead<'s> {
    fn new(c: &'s Adc, s: &'s Adc) -> AdcRead<'s> {
        AdcRead {
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

    fn process(&mut self, data: &mut Data) {
        let current = self.current.read() as i32;
        data.current_adc_raw = current;
        data.current_adc = Self::clip(current, 0, 4096);
        let setpoint = self.setpoint.read() as i32;
        data.setpoint_adc_raw = setpoint;
        data.setpoint_adc = Self::clip(setpoint, 0, 4096);
    }
}

#[derive(Default)]
pub struct Setpoint;

impl Setpoint {
    fn process(&mut self, data: &mut Data) {
        data.setpoint_mdeg = match data.setpoint_adc {
            // The log poti in the fridge is very hard to adjust, so
            // I use three predefined temperature ranges
            0...180 => 5000,
            181...660 => 10000,
            _ => 15000,
        }
    }
}

#[derive(Default)]
pub struct Current;

impl Current {
    fn process(&mut self, data: &mut Data) {
        // The temperature sensor fails by 4deg...
        data.current_mdeg = data.current_adc * 100 - 4000;
    }
}

struct AdcFilter {
    current_filter_plausible: PlausibleFilter,
    setpoint_filter_plausible: PlausibleFilter,
    current_filter: MeanFilter,
    setpoint_filter: MeanFilter,
}

impl<'s> AdcFilter {
    fn new(setpoint: i32, current: i32) -> AdcFilter {
        AdcFilter {
            current_filter_plausible: PlausibleFilter::new(5, 500),
            setpoint_filter_plausible: PlausibleFilter::new(5, 500),
            current_filter: MeanFilter::new(current),
            setpoint_filter: MeanFilter::new(setpoint),
        }
    }

    fn process(&mut self, data: &mut Data) {
        data.setpoint_adc = self.setpoint_filter_plausible.filter(data.setpoint_adc);
        data.setpoint_adc = self.setpoint_filter.filter(data.setpoint_adc);
        data.current_adc = self.current_filter_plausible.filter(data.current_adc);
        data.current_adc = self.current_filter.filter(data.current_adc);
    }
}

struct StateLed<'s> {
    on: bool,
    pin: &'s Gpio,
    run: u8,
}

impl<'s> StateLed<'s> {
    fn new(l: &'s Gpio) -> StateLed<'s> {
        StateLed {
            on: false,
            pin: l,
            run: 0,
        }
    }

    fn process(&mut self, data: &mut Data) {
        let rate = if data.compressor {
            1
        } else {
            10
        };

        if self.run % rate == 0 {
            self.on = match self.on {
                false => {
                    self.pin.set_high();
                    true
                }
                true => {
                    self.pin.set_low();
                    false
                }
            }
        }

        self.run = if self.run == 0xFE {
            0
        } else {
            self.run + 1
        }
    }
}

struct Control {
    hysteresis_mdeg: i32,
}

impl Control {
    fn new(hysteresis: i32) -> Control {
        Control { hysteresis_mdeg: hysteresis }
    }

    fn process(&mut self, data: &mut Data) {
        if data.current_mdeg >= (data.setpoint_mdeg + self.hysteresis_mdeg) {
            data.compressor = true;
        }
        if data.current_mdeg <= (data.setpoint_mdeg - self.hysteresis_mdeg) {
            data.compressor = false;
        }
    }
}

struct Compressor<'s> {
    pin: &'s Gpio,
}

impl<'s> Compressor<'s> {
    fn new(p: &'s Gpio) -> Compressor {
        Compressor { pin: p }
    }

    fn process(&mut self, data: &mut Data) {
        match data.compressor {
            false => self.pin.set_low(),
            true => self.pin.set_high(),
        }
    }
}

pub trait Tracer {
    fn trace(&mut self, data: &mut Data);
}

pub struct Trace<'s> {
    io: &'s CharIO,
}

impl<'s> Trace<'s> {
    pub fn new(cio: &'s CharIO) -> Trace {
        Trace { io: cio }
    }
}

impl<'s> Tracer for Trace<'s> {
    fn trace(&mut self, data: &mut Data) {
        match data.compressor {
            true => self.io.puts("[cooling]: "),
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

pub fn run(p: &Platform, tracer: &mut Tracer, loops: Option<u32>) {
    let mut data = Data::default();

    let mut adc_filter = AdcFilter::new(5, 10);
    let mut adc_input = AdcRead::new(p.current, p.setpoint);
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
        tracer.trace(&mut data);
    };

    // Add some start delay because the ADCs are failling directly after POR
    p.timer.wait_ms(5000);

    match loops {
        Some(n) => {
            for _ in 0..n {
                r();
            }
        } // this is for testing
        None => {
            loop {
                r();
                p.timer.wait_ms(100)
            }
        }
    }
}
