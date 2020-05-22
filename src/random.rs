use rand::distributions::uniform::SampleBorrow;
use rand::distributions::uniform::SampleUniform;

use rand::Rng;
use rand_pcg;

/// The rand struct exposes the `rand_int` function for number generation.
///
/// ```
/// use resmack::random::Rand;
///
/// let seed = 1337;
/// // must be mutable!
/// let mut rand = Rand::new(seed);
/// let res = rand.rand_int(-5, 5);
///
/// for _ in (0..100) {
///     assert_eq!(-5 <= res && res < 5, true);
/// }
/// ```
pub struct Rand {
    rng: rand_pcg::Pcg64Mcg,
}

impl Rand {
    pub fn new(seed: u128) -> Rand {
        Rand {
            rng: rand_pcg::Pcg64Mcg::new(seed),
        }
    }

    /// Generates a new value in the range `[min, max)`
    pub fn rand_int<T: SampleUniform, B1, B2>(&mut self, min: B1, max: B2) -> T
    where
        B1: SampleBorrow<T> + Sized,
        B2: SampleBorrow<T> + Sized,
    {
        self.rng.gen_range(min, max)
    }
}
