use fnv::FnvHasher;
use std::collections::{BTreeMap, HashMap};
use std::hash::BuildHasherDefault;
use twox_hash::XxHash64;

// see http://cglab.ca/~abeinges/blah/hash-rs/ for comparisons of different
// hash functions (and BTreeMap!)
#[allow(dead_code)]
pub type FastLongHash<T, U> = HashMap<T, U, BuildHasherDefault<XxHash64>>;
#[allow(dead_code)]
pub type FastMedHash<T, U> = HashMap<T, U, BuildHasherDefault<FnvHasher>>;
#[allow(dead_code)]
pub type FastShortHash<T, U> = BTreeMap<T, U>;

#[allow(dead_code)]
pub fn new_fast_long_hash<T, U>() -> FastLongHash<T, U> {
    let hash: HashMap<T, U, BuildHasherDefault<XxHash64>> = Default::default();
    hash
}

#[allow(dead_code)]
pub fn new_fast_med_hash<T, U>() -> FastMedHash<T, U> {
    let hash: HashMap<T, U, BuildHasherDefault<FnvHasher>> = Default::default();
    hash
}

pub fn new_fast_short_hash<T, U>() -> FastShortHash<T, U>
where
    T: std::cmp::Ord,
{
    let hash: BTreeMap<T, U> = BTreeMap::new();
    hash
}
