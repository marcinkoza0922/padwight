//! What the rasterizer keeps between frames, within a budget of bytes: the least recently used
//! go first once the rest pass it.

use std::{collections::HashMap, hash::Hash, rc::Rc};

pub(super) struct Kept<K, V> {
    items: HashMap<K, (Option<Rc<V>>, usize, u64)>,
    bytes: usize,
    budget: usize,
    clock: u64,
}

impl<K: Hash + Eq + Clone, V> Kept<K, V> {
    pub fn new(budget: usize) -> Self {
        Kept { items: HashMap::new(), bytes: 0, budget, clock: 0 }
    }

    /// What `key` names, made by `make` the first time with its size in bytes. A failure is kept
    /// too, so a file that won't load isn't read again every frame.
    pub fn get(&mut self, key: &K, make: impl FnOnce() -> Option<(V, usize)>) -> Option<Rc<V>> {
        self.clock += 1;
        let clock = self.clock;
        if let Some((value, _, used)) = self.items.get_mut(key) {
            *used = clock;
            return value.clone();
        }
        let (value, bytes) = make().map_or((None, 0), |(v, bytes)| (Some(Rc::new(v)), bytes));
        self.bytes += bytes;
        while self.bytes > self.budget {
            let Some(oldest) = self.items.iter().min_by_key(|(_, (_, _, used))| *used).map(|(k, _)| k.clone()) else { break };
            if let Some((_, gone, _)) = self.items.remove(&oldest) {
                self.bytes -= gone;
            }
        }
        self.items.insert(key.clone(), (value.clone(), bytes, clock));
        value
    }

    /// How many are kept, failures included.
    #[cfg(test)]
    pub fn len(&self) -> usize {
        self.items.len()
    }

    pub fn clear(&mut self) {
        self.items.clear();
        self.bytes = 0;
    }
}
