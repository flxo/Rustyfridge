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

#![allow(dead_code)]

/// Filter
pub mod filter {
    pub trait Filter {
        fn filter(&mut self, value: i32) -> i32;
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
    }

    impl Filter for MeanFilter {
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
    }

    impl Filter for PlausibleFilter {
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
}
