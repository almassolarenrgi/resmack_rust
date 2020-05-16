use fnv::FnvHasher;
use std::collections::HashMap;
use std::hash::BuildHasherDefault;
use twox_hash::XxHash64;

pub type FastLongHash<T, U> = HashMap<T, U, BuildHasherDefault<XxHash64>>;
pub type FastShortHash<T, U> = HashMap<T, U, BuildHasherDefault<FnvHasher>>;

pub fn new_fast_long_hash<T, U>() -> FastLongHash<T, U> {
    let hash: HashMap<T, U, BuildHasherDefault<XxHash64>> = Default::default();
    hash
}

pub fn new_fast_short_hash<T, U>() -> FastShortHash<T, U> {
    let hash: HashMap<T, U, BuildHasherDefault<FnvHasher>> = Default::default();
    hash
}
