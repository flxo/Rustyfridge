pub trait Filter {
    fn filter(&self, value: i32) -> i32;
}

pub struct MeanFilter {
    last: i32,
    num: u32,
}

impl MeanFilter {
    pub fn new(num: u32, init: Option<i32>) -> MeanFilter {
        let i = match init {
            Some(v) => v,
            None => 0,
        };
        MeanFilter {
            last: i,
            num: num,
        }
    }
}

impl Filter for MeanFilter {
    fn filter(&self, value: i32) -> i32 {
        let n = self.num as i32;
        (self.last * (n-1) + value) / n
    }
}
