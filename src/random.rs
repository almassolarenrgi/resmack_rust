use rand::Rng;
//use rand::SeedableRng;
use rand_pcg;

pub struct Rand {
    rng: rand_pcg::Pcg64Mcg,
}

impl Rand {
    pub fn new(seed: u128) -> Rand {
        Rand {
            rng: rand_pcg::Pcg64Mcg::new(seed),
        }
    }

    pub fn rand_int(&mut self, min: usize, max: usize) -> usize {
        self.rng.gen_range(min, max)
    }
}
