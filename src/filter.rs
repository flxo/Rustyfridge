pub mod filter {

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

    #[allow(dead_code)]
    pub struct DiffFilter {
        last: Option<i32>,
    }

    #[allow(dead_code)]
    impl DiffFilter {
        pub fn new() -> DiffFilter {
            DiffFilter {
                last: None,
            }
        }
    }

    impl Filter for DiffFilter {
        fn filter(&mut self, value: i32) -> i32 {
            value
        }
    }

    #[allow(dead_code)]
    pub struct PlausibleFilter {
        num_fails: u16,
        fails: u16,
        diff: i32,
        last: Option<i32>,
    }

     #[allow(dead_code)]
    impl PlausibleFilter {
        pub fn new(n: u16, d: i32) -> PlausibleFilter {
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
                        self.last = Some(value);
                        value
                    } else {
                        self.num_fails += 1;
                        if self.fails > self.num_fails {
                            self.num_fails = 0;
                            self.last = Some(value);
                            value
                        } else {
                            self.last.unwrap()
                        }
                    }
                },
                None => { self.last = Some(value); value },
            }
        }
    }
}
