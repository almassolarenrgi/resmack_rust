//use num::{Num, NumCast, ToPrimitive};

/// The rand struct exposes the `rand_int` function for number generation.
///
/// ```
/// use resmack::random::Rand;
///
/// let seed = 1337;
/// // must be mutable!
/// let mut rand = Rand::new(seed);
/// let res = rand.rand_i64(-5, 5);
///
/// for _ in (0..100) {
///     assert_eq!(-5 <= res && res < 5, true);
/// }
/// ```
pub struct Rand {
    seed: [u64; 2],
}

impl Rand {
    pub fn new(seed: u128) -> Rand {
        Rand {
            seed: [
                (seed & ((1 << 64) - 1)) as u64,
                ((seed >> 64) & ((1 << 64) - 1)) as u64,
            ],
        }
    }

    /// Generates a new value in the range `[min, max)`
    pub fn rand_u64(&mut self, min: u64, max: u64) -> u64 {
        let num = self.next();
        let diff = max - min;
        let res = num % diff;
        res + min
    }

    /// Generates a new value in the range `[min, max)`
    pub fn rand_i64(&mut self, min: i64, max: i64) -> i64 {
        let num = self.next();
        let diff: u64 = (max - min) as u64;
        let res = num % diff;
        (res as i64) + min
    }

    fn next(&mut self) -> u64 {
        let s0 = self.seed[0];
        let mut s1 = self.seed[1];
        let result = s0.wrapping_mul(5).rotate_left(7).wrapping_mul(9);

        s1 ^= s0;
        self.seed[0] = s0.rotate_left(24) ^ s1 ^ (s1 << 16);
        self.seed[1] = s1.rotate_left(37);

        result
    }
}

/*
 * http://prng.di.unimi.it/xoroshiro128starstar.c
 *

/*  Written in 2018 by David Blackman and Sebastiano Vigna (vigna@acm.org)

To the extent possible under law, the author has dedicated all copyright
and related and neighboring rights to this software to the public domain
worldwide. This software is distributed without any warranty.

See <http://creativecommons.org/publicdomain/zero/1.0/>. */

#include <stdint.h>

/* This is xoroshiro128** 1.0, one of our all-purpose, rock-solid,
   small-state generators. It is extremely (sub-ns) fast and it passes all
   tests we are aware of, but its state space is large enough only for
   mild parallelism.

   For generating just floating-point numbers, xoroshiro128+ is even
   faster (but it has a very mild bias, see notes in the comments).

   The state must be seeded so that it is not everywhere zero. If you have
   a 64-bit seed, we suggest to seed a splitmix64 generator and use its
   output to fill s. */


static inline uint64_t rotl(const uint64_t x, int k) {
    return (x << k) | (x >> (64 - k));
}


static uint64_t s[2];

uint64_t next(void) {
    const uint64_t s0 = s[0];
    uint64_t s1 = s[1];
    const uint64_t result = rotl(s0 * 5, 7) * 9;

    s1 ^= s0;
    s[0] = rotl(s0, 24) ^ s1 ^ (s1 << 16); // a, b
    s[1] = rotl(s1, 37); // c

    return result;
}


/* This is the jump function for the generator. It is equivalent
   to 2^64 calls to next(); it can be used to generate 2^64
   non-overlapping subsequences for parallel computations. */

void jump(void) {
    static const uint64_t JUMP[] = { 0xdf900294d8f554a5, 0x170865df4b3201fc };

    uint64_t s0 = 0;
    uint64_t s1 = 0;
    for(int i = 0; i < sizeof JUMP / sizeof *JUMP; i++)
        for(int b = 0; b < 64; b++) {
            if (JUMP[i] & UINT64_C(1) << b) {
                s0 ^= s[0];
                s1 ^= s[1];
            }
            next();
        }

    s[0] = s0;
    s[1] = s1;
}


/* This is the long-jump function for the generator. It is equivalent to
   2^96 calls to next(); it can be used to generate 2^32 starting points,
   from each of which jump() will generate 2^32 non-overlapping
   subsequences for parallel distributed computations. */

void long_jump(void) {
    static const uint64_t LONG_JUMP[] = { 0xd2a98b26625eee7b, 0xdddf9b1090aa7ac1 };

    uint64_t s0 = 0;
    uint64_t s1 = 0;
    for(int i = 0; i < sizeof LONG_JUMP / sizeof *LONG_JUMP; i++)
        for(int b = 0; b < 64; b++) {
            if (LONG_JUMP[i] & UINT64_C(1) << b) {
                s0 ^= s[0];
                s1 ^= s[1];
            }
            next();
        }

    s[0] = s0;
    s[1] = s1;
}

 */
