// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// https://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// https://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or https://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! A wrapper around another RNG that reseeds it after it
//! generates a certain number of random bytes.

use core::default::Default;

use {Rng, Error};

/// A wrapper around any RNG which reseeds the underlying RNG after it
/// has generated a certain number of random bytes.
#[derive(Debug)]
pub struct ReseedingRng<R, Rsdr> {
    rng: R,
    generation_threshold: u64,
    bytes_generated: u64,
    /// Controls the behaviour when reseeding the RNG.
    pub reseeder: Rsdr,
}

impl<R: Rng, Rsdr: Reseeder<R>> ReseedingRng<R, Rsdr> {
    /// Create a new `ReseedingRng` with the given parameters.
    ///
    /// # Arguments
    ///
    /// * `rng`: the random number generator to use.
    /// * `generation_threshold`: the number of bytes of entropy at which to reseed the RNG.
    /// * `reseeder`: the reseeding object to use.
    pub fn new(rng: R, generation_threshold: u64, reseeder: Rsdr) -> ReseedingRng<R,Rsdr> {
        ReseedingRng {
            rng: rng,
            generation_threshold: generation_threshold,
            bytes_generated: 0,
            reseeder: reseeder
        }
    }

    /// Reseed the internal RNG if the number of bytes that have been
    /// generated exceed the threshold.
    pub fn reseed_if_necessary(&mut self) {
        if self.bytes_generated >= self.generation_threshold {
            self.reseeder.reseed(&mut self.rng);
            self.bytes_generated = 0;
        }
    }
}


impl<R: Rng, Rsdr: Reseeder<R>> Rng for ReseedingRng<R, Rsdr> {
    fn next_u32(&mut self) -> u32 {
        self.reseed_if_necessary();
        self.bytes_generated += 4;
        self.rng.next_u32()
    }

    fn next_u64(&mut self) -> u64 {
        self.reseed_if_necessary();
        self.bytes_generated += 8;
        self.rng.next_u64()
    }

    fn fill_bytes(&mut self, dest: &mut [u8]) {
        self.reseed_if_necessary();
        self.bytes_generated += dest.len() as u64;
        self.rng.fill_bytes(dest)
    }
    
    fn try_fill_bytes(&mut self, dest: &mut [u8]) -> Result<(), Error> {
        self.reseed_if_necessary();
        self.bytes_generated += dest.len() as u64;
        self.rng.try_fill_bytes(dest)
    }
}

/// Something that can be used to reseed an RNG via `ReseedingRng`.
///
/// Note that implementations should support `Clone` only if reseeding is
/// deterministic (no external entropy source). This is so that a `ReseedingRng`
/// only supports `Clone` if fully deterministic.
pub trait Reseeder<R> {
    /// Reseed the given RNG.
    fn reseed(&mut self, rng: &mut R);
}

/// Reseed an RNG using a `Default` instance. This reseeds by
/// replacing the RNG with the result of a `Default::default` call.
#[derive(Clone, Copy, Debug)]
pub struct ReseedWithDefault;

impl<R: Rng + Default> Reseeder<R> for ReseedWithDefault {
    fn reseed(&mut self, rng: &mut R) {
        *rng = Default::default();
    }
}
impl Default for ReseedWithDefault {
    fn default() -> ReseedWithDefault { ReseedWithDefault }
}

#[cfg(test)]
mod test {
    use {impls, le};
    use core::default::Default;
    use super::{ReseedingRng, ReseedWithDefault};
    use {SeedableRng, Rng};

    struct Counter {
        i: u32
    }

    impl Rng for Counter {
        fn next_u32(&mut self) -> u32 {
            self.i += 1;
            // very random
            self.i - 1
        }
        fn next_u64(&mut self) -> u64 {
            impls::next_u64_via_u32(self)
        }

        fn fill_bytes(&mut self, dest: &mut [u8]) {
            impls::fill_bytes_via_u64(self, dest)
        }
    }
    impl Default for Counter {
        fn default() -> Counter {
            Counter { i: 0 }
        }
    }
    impl SeedableRng for Counter {
        type Seed = [u8; 4];
        fn from_seed(seed: Self::Seed) -> Self {
            let mut seed_u32 = [0u32; 1];
            le::read_u32_into(&seed, &mut seed_u32);
            Counter { i: seed_u32[0] }
        }
    }
    type MyRng = ReseedingRng<Counter, ReseedWithDefault>;

    #[test]
    fn test_reseeding() {
        let mut rs = ReseedingRng::new(Counter {i:0}, 400, ReseedWithDefault);

        let mut i = 0;
        for _ in 0..1000 {
            assert_eq!(rs.next_u32(), i % 100);
            i += 1;
        }
    }

    const FILL_BYTES_V_LEN: usize = 13579;
    #[test]
    fn test_rng_fill_bytes() {
        let mut v = [0u8; FILL_BYTES_V_LEN];
        ::test::rng(321).fill_bytes(&mut v);

        // To test that `fill_bytes` actually did something, check that not all
        // bytes are zero.
        assert!(!v.iter().all(|&x| x == 0));
    }
}
