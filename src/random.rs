use rand::distributions::uniform::SampleBorrow;
use rand::distributions::uniform::SampleUniform;

use rand::Rng;
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

    pub fn rand_int<T: SampleUniform, B1, B2>(&mut self, min: B1, max: B2) -> T
    where
        B1: SampleBorrow<T> + Sized,
        B2: SampleBorrow<T> + Sized,
    {
        self.rng.gen_range(min, max)
    }
}
