use rand::Rng;

pub struct Rand {}

impl Rand {
    pub fn rand_int(min: usize, max: usize) -> usize {
        rand::thread_rng().gen_range(min, max)
    }
}
