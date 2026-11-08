//! Bộ nhớ đệm LRU giới hạn theo số byte (design.md: bộ nhớ đệm ảnh trang 300 MB).

use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;

/// Giới hạn mặc định của bộ nhớ đệm ảnh trang.
pub const PAGE_CACHE_BYTES: usize = 300 * 1024 * 1024;

pub struct ByteLru<K, V> {
    capacity: usize,
    used: usize,
    tick: u64,
    map: HashMap<K, (V, usize, u64)>,
    order: BTreeMap<u64, K>,
}

impl<K: Hash + Eq + Clone, V> ByteLru<K, V> {
    pub fn new(capacity_bytes: usize) -> Self {
        Self { capacity: capacity_bytes, used: 0, tick: 0, map: HashMap::new(), order: BTreeMap::new() }
    }

    pub fn capacity(&self) -> usize {
        self.capacity
    }

    /// Tổng số byte đang giữ.
    pub fn used_bytes(&self) -> usize {
        self.used
    }

    pub fn len(&self) -> usize {
        self.map.len()
    }

    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }

    pub fn contains(&self, k: &K) -> bool {
        self.map.contains_key(k)
    }

    /// Lấy giá trị và đánh dấu là vừa dùng.
    pub fn get(&mut self, k: &K) -> Option<&V> {
        self.tick += 1;
        let t = self.tick;
        let entry = self.map.get_mut(k)?;
        self.order.remove(&entry.2);
        entry.2 = t;
        self.order.insert(t, k.clone());
        Some(&entry.0)
    }

    /// Xem mà không đổi thứ tự.
    pub fn peek(&self, k: &K) -> Option<&V> {
        self.map.get(k).map(|e| &e.0)
    }

    /// Thêm giá trị có kích thước `bytes`; bỏ các mục cũ nhất cho tới khi vừa giới hạn.
    /// Trả về các mục bị bỏ.
    pub fn insert(&mut self, k: K, v: V, bytes: usize) -> Vec<(K, V)> {
        let mut evicted = Vec::new();
        if let Some(old) = self.remove(&k) {
            drop(old);
        }
        while self.used + bytes > self.capacity {
            match self.pop_oldest() {
                Some(e) => evicted.push(e),
                None => break,
            }
        }
        if bytes > self.capacity {
            // Một mục lớn hơn cả giới hạn thì không giữ.
            evicted.push((k, v));
            return evicted;
        }
        self.tick += 1;
        self.order.insert(self.tick, k.clone());
        self.map.insert(k, (v, bytes, self.tick));
        self.used += bytes;
        evicted
    }

    pub fn remove(&mut self, k: &K) -> Option<V> {
        let (v, size, t) = self.map.remove(k)?;
        self.order.remove(&t);
        self.used -= size;
        Some(v)
    }

    fn pop_oldest(&mut self) -> Option<(K, V)> {
        let (&t, _) = self.order.iter().next()?;
        let k = self.order.remove(&t)?;
        let (v, size, _) = self.map.remove(&k)?;
        self.used -= size;
        Some((k, v))
    }

    /// Bỏ mọi mục không thỏa điều kiện.
    pub fn retain(&mut self, mut keep: impl FnMut(&K) -> bool) {
        let drop_keys: Vec<K> = self.map.keys().filter(|k| !keep(k)).cloned().collect();
        for k in drop_keys {
            self.remove(&k);
        }
    }

    pub fn clear(&mut self) {
        self.map.clear();
        self.order.clear();
        self.used = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn evicts_least_recently_used() {
        let mut c = ByteLru::new(100);
        c.insert(1, "a", 40);
        c.insert(2, "b", 40);
        c.get(&1);
        let ev = c.insert(3, "c", 40);
        assert_eq!(ev.len(), 1);
        assert_eq!(ev[0].0, 2);
        assert!(c.contains(&1) && c.contains(&3));
        assert_eq!(c.used_bytes(), 80);
    }

    #[test]
    fn never_exceeds_capacity() {
        let mut c = ByteLru::new(1000);
        for i in 0..10_000u32 {
            c.insert(i, (), 1 + (i as usize * 7) % 300);
            assert!(c.used_bytes() <= 1000);
        }
        c.insert(99_999, (), 5000);
        assert!(!c.contains(&99_999));
    }
}
