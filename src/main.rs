#![feature(alloc)]
#![no_std]

extern crate alloc;
extern crate tock;

use alloc::string::String;
use tock::console::Console;
use tock::led;
use tock::syscalls;
use tock::timer;
use tock::timer::Duration;

const ADC_DRIVER: usize = 0x00005;
const SUBSCRIBE: usize = 0;
const SAMPLE: usize = 1;

fn log(s: &str, v: Option<i32>) {
    let mut c = Console::new();
    c.write(String::from(s));
    if let Some(v) = v {
        c.write(String::from(" "));
        c.write(tock::fmt::i32_as_decimal(v as i32));
    }
    c.write(String::from("\n"));
}

fn sample(channel: usize) -> isize {
    unsafe { syscalls::command(ADC_DRIVER, SAMPLE, channel, 0) }
}

use core::convert::From;
use core::ops::{Add, Mul};

#[derive(Default)]
struct FloatMean<T>(Option<T>);

impl<T> FloatMean<T>
where
    T: Mul<i32> + Add<T, Output = T> + From<i32> + Mul<T, Output = T>,
    T: Add<T>,
{
    fn add(&mut self, other: T) {
        match self.0 {
            Some(v) => {
                self.0 = {
                    let a = v * 19;
                    let b = a + other;
                    Some(b / 20)
                }
            }
            None => self.0 = Some(other),
        }
    }

    fn value(&self) -> T {
        self.0.unwrap_or(0.into())
    }
}

fn main() {
    let mut setpoint = FloatMean::default();
    let mut current = FloatMean::default();

    let mut cb = |_, c, v| {
        led::get(0).unwrap().toggle();
        if c == 0 {
            setpoint.add(v as i32);
            sample(1);
        } else {
            current.add(v as i32);
            log("Setpoint", Some(setpoint.value()));
            log("Current", Some(current.value()));
        }
    };
    let _subscription = syscalls::subscribe(ADC_DRIVER, SUBSCRIBE, &mut cb).unwrap();

    loop {
        sample(0);
        led::get(1).unwrap().toggle();
        timer::sleep(Duration::from_ms(100));
    }
}
