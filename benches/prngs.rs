extern crate rand;

use criterion::{criterion_group, criterion_main, Criterion};

use rand::rngs::{OsRng, StdRng};
use rand::{Rng, RngCore, SeedableRng};
use rand_chacha::ChaChaRng;
use rand_hc::Hc128Rng;
use rand_isaac::IsaacRng;
use rand_xorshift::XorShiftRng;

// ----------------------------------------------------------------------------

pub struct Xoshiro128StarStar {
    seed: [u64; 2],
}

impl Xoshiro128StarStar {
    pub fn new(seed: u64) -> Xoshiro128StarStar {
        let mut rng: StdRng = SeedableRng::seed_from_u64(seed);
        let seed: u128 = rng.gen();

        Xoshiro128StarStar {
            seed: [
                (seed & ((1 << 64) - 1)) as u64,
                ((seed >> 64) & ((1 << 64) - 1)) as u64,
            ],
        }
    }

    pub fn next(&mut self) -> u64 {
        let s0 = self.seed[0];
        let mut s1 = self.seed[1];
        let result = s0.wrapping_mul(5).rotate_left(7).wrapping_mul(9);

        s1 ^= s0;
        self.seed[0] = s0.rotate_left(24) ^ s1 ^ (s1 << 16);
        self.seed[1] = s1.rotate_left(37);

        result
    }
}

// ----------------------------------------------------------------------------

pub struct FZeroRand {
    seed: usize,
}

impl FZeroRand {
    pub fn new(seed: u64) -> FZeroRand {
        let mut rng: StdRng = SeedableRng::seed_from_u64(seed);
        let seed: usize = rng.gen();

        FZeroRand {
            seed: seed as usize,
        }
    }

    pub fn next(&mut self) -> u64 {
        let mut seed = self.seed;
        seed ^= seed << 13;
        seed ^= seed >> 17;
        seed ^= seed << 43;
        self.seed = seed;
        seed as u64
    }
}

// ----------------------------------------------------------------------------

pub fn compare_prngs(c: &mut Criterion) {
    let mut group = c.benchmark_group("prngs");
    group.bench_function("Xoshiro128**", |b| {
        let mut test_rand = Xoshiro128StarStar::new(1337);
        b.iter(|| test_rand.next());
    });
    group.bench_function("FZero", |b| {
        let mut test_rand = FZeroRand::new(1337);
        b.iter(|| test_rand.next());
    });
    group.bench_function("rand_xorshift", |b| {
        let mut test_rand = XorShiftRng::seed_from_u64(1337);
        b.iter(|| test_rand.next_u64());
    });
    group.bench_function("rand::XorShiftRng", |b| {
        let mut test_rand: XorShiftRng = SeedableRng::seed_from_u64(1337);
        b.iter(|| test_rand.next_u64());
    });
    group.bench_function("rand::ChaCha", |b| {
        let mut test_rand: ChaChaRng = SeedableRng::seed_from_u64(1337);
        b.iter(|| test_rand.next_u64());
    });
    group.bench_function("rand::Hc128", |b| {
        let mut test_rand: Hc128Rng = SeedableRng::seed_from_u64(1337);
        b.iter(|| test_rand.next_u64());
    });
    group.bench_function("rand::IsaacRng", |b| {
        let mut test_rand: IsaacRng = SeedableRng::seed_from_u64(1337);
        b.iter(|| test_rand.next_u64());
    });
    group.bench_function("rand::StdRng", |b| {
        let mut test_rand: StdRng = SeedableRng::seed_from_u64(1337);
        b.iter(|| test_rand.next_u64());
    });
    // seriously skews the graph - 600x slower than Xoshiro128**
    group.bench_function("rand::OsRng", |b| {
        b.iter(|| OsRng.next_u64());
    });
    group.finish();
}

criterion_group!(benches, compare_prngs);
criterion_main!(benches);
