use crate::storages::Version;
use std::cmp::Ordering;
use std::hash::{Hash, Hasher};
use std::marker::PhantomData;

impl<T> PartialEq for Version<T> {
    fn eq(&self, other: &Self) -> bool {
        self.version == other.version
    }
}

impl<T> Eq for Version<T> {}

impl<T> Ord for Version<T> {
    fn cmp(&self, other: &Self) -> Ordering {
        self.version.cmp(&other.version)
    }
}

impl<T> PartialOrd for Version<T> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.version.partial_cmp(&other.version)
    }
}

impl<T> Hash for Version<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.version.hash(state)
    }
}

impl<T> Default for Version<T> {
    fn default() -> Self {
        Self {
            version: u64::default(),
            marker: PhantomData,
        }
    }
}

impl<T> Clone for Version<T> {
    fn clone(&self) -> Self {
        Self {
            version: self.version,
            marker: PhantomData,
        }
    }
}

impl<T> Copy for Version<T> {}

impl<T> Version<T> {
    pub fn new() -> Self {
        Self {
            version: 0,
            marker: PhantomData,
        }
    }

    pub fn next(&self) -> Self {
        let new_rev = self
            .version
            .checked_add(1)
            .expect("Revision overflowed u64");
        Self {
            version: new_rev,
            ..*self
        }
    }

    pub fn advance(&mut self) {
        *self = self.next()
    }
}
