use zinc::hal::lpc17xx::{pin};
use zinc::hal::pin::{Adc, Gpio};
use zinc::drivers::chario::CharIO;

#[derive(Default)]
pub struct Data {
    setpoint_adc: i32,
    setpoint_mdeg: i32,
    current_adc: i32,
    current_mdeg: i32,
    compressor: bool,
}

pub trait Step {
    fn process(&mut self, data: &mut Data);
}

pub trait Filter {
    fn filter(&mut self, value: i32) -> i32;
}

pub struct MeanFilter {
    last: Option<i32>,
    num: i32,
}

impl MeanFilter {
    pub fn new(num: i32) -> MeanFilter {
        MeanFilter {
            last: None,
            num: num,
        }
    }
}

impl Filter for MeanFilter {
    fn filter(&mut self, value: i32) -> i32 {
        self.last = match self.last {
            Some(l) => Some((l * (self.num - 1) + value) / self.num),
            None    => Some(value),
        };
        self.last.unwrap()
    }
}

pub struct AdcRead<'s> {
    current: &'s pin::Pin,
    setpoint: &'s pin::Pin,
}

impl<'s> AdcRead<'s> {
    pub fn new(c: &'s pin::Pin, s: &'s pin::Pin) -> AdcRead<'s> {
        AdcRead {
            current: c,
            setpoint: s,
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

impl<'s> Step for AdcRead<'s> {
    fn process(&mut self, data: &mut Data) {
        let current = self.current.read() as i32;
        data.current_adc = self.clip(current, 0, 4096);
        let setpoint = self.setpoint.read() as i32;
        data.setpoint_adc = self.clip(setpoint, 0, 4096);
    }
}

#[derive(Default)]
pub struct Setpoint;

impl Step for Setpoint {
    fn process(&mut self, data: &mut Data) {
        data.setpoint_mdeg = match data.setpoint_adc {
            0...180   => 5000,
            181...660 => 10000,
            _         => 15000,
        }
    }
}

#[derive(Default)]
pub struct Current;

impl Step for Current {
    fn process(&mut self, data: &mut Data) {
        // the used sensor fails by 4deg...
        data.current_mdeg = data.current_adc * 100 - 4000;
    }
}

pub struct AdcFilter {
    current_filter: MeanFilter,
    setpoint_filter: MeanFilter,
}

impl AdcFilter {
    pub fn new(setpoint: i32, current: i32) -> AdcFilter {
        AdcFilter {
            current_filter: MeanFilter::new(current),
            setpoint_filter: MeanFilter::new(setpoint),
        }
    }
}

impl Step for AdcFilter {
    fn process(&mut self, data: &mut Data) {
        data.setpoint_adc = self.setpoint_filter.filter(data.setpoint_adc);
        data.current_adc = self.current_filter.filter(data.current_adc);
    }
}

pub struct StateLed<'s> {
    on: bool,
    pin: &'s pin::Pin,
}

impl<'s> StateLed<'s> {
    pub fn new(l: &'s pin::Pin) -> StateLed<'s> {
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

pub struct Control {
    hysteresis_mdeg: i32,
}

impl Control {
    pub fn new(hysteresis: i32) -> Control {
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

pub struct Compressor<'s> {
    pin: &'s pin::Pin,
}

impl<'s> Compressor<'s> {
    pub fn new(p: &'s pin::Pin) -> Compressor {
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

    fn print_deg(&self, value: i32) {
        let v = value as u32;
        self.io.puti(v / 1000);
        self.io.puts(".");
        self.io.puti(v % 1000);
        self.io.puts(" deg");
    }
}

impl<'s> Step for Trace<'s> {
    fn process(&mut self, data: &mut Data) {
        match data.compressor {
            true  => self.io.puts("[cooling]: "),
            false => self.io.puts("[stopped]: "),
        }
        self.io.puts("setpoint: ");
        self.print_deg(data.setpoint_mdeg);
        self.io.puts("\t");
        self.io.puts("current: ");
        self.print_deg(data.current_mdeg);
    }
}
