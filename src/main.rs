#![feature(start, plugin, no_std, core_intrinsics)]
#![no_std]
#![plugin(macro_platformtree)]

extern crate zinc;
use zinc::hal::lpc17xx::{pin};
use zinc::hal::pin::Adc;
use zinc::hal::pin::Gpio;

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
        led@22 { direction = "out"; }
        compressor@8 { direction = "out"; }
      }
    }
  }

  os {
    single_task {
      loop = "run";
      args {
        timer = &timer;
        uart = &uart;
        led = &led;
        compressor = &compressor;
      }
    }
  }
);

/*
fn clip(value: i32, min: i32, max: i32) -> i32 {
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

fn norm(value: i32, min: i32, max: i32, min_norm: i32, max_norm: i32) -> i32 {
    let v = value - min;
    let tr = max_norm - min_norm;
    let ir = max - min;
    (((v * tr) / ir) + min_norm)
}
*/

fn run(args: &pt::run_args) {
  use zinc::drivers::chario::CharIO;
  use zinc::hal::timer::Timer;

  args.uart.puts("starting\n");

  let setpoint_adc = pin::Pin::new(
    pin::Port::Port0, 23,
    pin::Function::AltFunction1,
    None);
  let current_adc = pin::Pin::new(
    pin::Port::Port0, 25,
    pin::Function::AltFunction1,
    None);

  let mut led_state = false;
  let mut compressor_state = false;
  let hysteresis_mdeg = 1500;

  loop {
      let setpoint_mdeg = match setpoint_adc.read() {
          0...180 => 5000,
          181...660 => 10000,
          _ => 15000,
      };

      let current_mdeg = current_adc.read() * 100 - 4000;

      if compressor_state {
        args.uart.puts("[running] ");
      } else {
        args.uart.puts("[stopped] ");
      }

      args.uart.puts("setpoint: ");
      args.uart.puti(setpoint_mdeg);
      args.uart.puts(" mdeg\tcurrent: ");
      args.uart.puti(current_mdeg);
      args.uart.puts(" mdeg\n");

      if led_state {
        args.led.set_high();
      } else {
        args.led.set_low();
      }

      led_state = !led_state;

      if current_mdeg >= (setpoint_mdeg + hysteresis_mdeg) {
          if !compressor_state {
            compressor_state = true;
            args.compressor.set_high();
          }
      } else {
          if compressor_state {
            compressor_state = false;
            args.compressor.set_low();
          }
      }
      args.timer.wait(1);
  }
}
