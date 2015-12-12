pub trait Filter {
    fn filter(&mut self, value: i32) -> i32;
}

pub struct MeanFilter {
    last: i32,
    num: u32,
}

impl MeanFilter {
    pub fn new(num: u32, init: i32) -> MeanFilter {
        MeanFilter {
            last: init,
            num: num,
        }
    }
}

impl Filter for MeanFilter {
    fn filter(&mut self, value: i32) -> i32 {
        let n = self.num as i32;
        self.last = (self.last * (n-1) + value) / n;
        self.last
    }
}
