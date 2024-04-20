use std::{collections::HashMap, fmt::Debug, hash::Hash};
use serde::{Deserialize, Serialize};

///
/// A storage for path-based entries
/// 
#[derive(Deserialize, Serialize)]
pub struct Cache<T> {
    sub_caches: HashMap<String, Cache<T>>,
    entries: HashMap<String, T>,
}

impl<T> Debug for Cache<T> where T : Debug {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Cache").field("sub_caches", &self.sub_caches).field("entries", &self.entries).finish()
    }
}

impl<T> Cache<T> {
    ///
    /// Creates a new, empty Cache
    /// 
    pub fn new() -> Self {
        Self { sub_caches: HashMap::new(), entries: HashMap::new() }
    }

    ///
    /// Inserts the provided entry into the `Cache` with the given path and name
    /// 
    pub fn insert(&mut self, path: &str, entr: T) {
        let mut vals = path.splitn(2, '/');
        let (pfx, sfx) = (vals.next().unwrap(), vals.next());

        if let Some(sfx) = sfx {
            if !self.sub_caches.contains_key(pfx) {
                self.sub_caches.insert(pfx.to_string(), Cache::new());
            }
            self.sub_caches.get_mut(pfx).unwrap().insert(sfx, entr);
        } else {
            self.entries.insert(pfx.to_string(), entr);
        }
    }
    ///
    /// Gets the value of entry found at the given entry path
    /// 
    pub fn get(&self, entr_path: &str) -> Option<&T> {
        let mut vals = entr_path.splitn(2, '/');
        let (pfx, sfx) = (vals.next().unwrap(), vals.next());

        if let Some(sfx) = sfx {
            return match self.sub_caches.get(pfx) {
                None => None,
                Some(cache) => cache.get(sfx)
            };
        }
        self.entries.get(pfx).and_then(|s| Some(s))
    }
    ///
    /// Removes the item, whether sub-Cache or entry, from the Cache, if found.
    /// Returns the item if it was found.
    /// 
    pub fn remove(&mut self, entr: &str) -> Option<T> {
        let mut vals = entr.splitn(2, '/');
        let (pfx, sfx) = (vals.next().unwrap(), vals.next());

        if let Some(sfx) = sfx {
            return if let Some(cache) = self.sub_caches.get_mut(pfx) {
                cache.remove(sfx)
            } else {
                None
            }
        }

        // If the end of the path has been reached, remove either 
        // Cache or entry, depending on which matches the keys
        // (if either do).
        if let None = self.sub_caches.remove(pfx) {
            return self.entries.remove(pfx);
        }
        None
    }
}

pub trait GroupBy<K : Eq + Hash, I> : IntoIterator<Item = I> {
    fn group_by(
        self, 
        whr: fn(&I) -> K
    ) -> HashMap<K, Vec<I>> where Self: Sized {
        let mut map = HashMap::<K, Vec<I>>::new();
        for item in self {
            let key = whr(&item);
            if let Some(list) = map.get_mut(&key) { 
                list.push(item);
            } else {
                let mut list = Vec::new();
                list.push(item);
                map.insert(key, list); 
            }
        }

        map
    }
}

impl<T, K : Eq + Hash, I> GroupBy<K, I> for T where T : IntoIterator<Item = I> { }

#[cfg(test)]
mod tests {
    use super::Cache;

    #[test]
    fn test_cache_insert() {
        let mut cache = Cache::new();
        cache.insert("path/to/entry", "me!");
        cache.insert("path/to/other_entry", "me too!");
        cache.insert("path/to/entry/1_level_lower", "I'm lower!");

        assert_eq!(cache.entries.len(), 0);
        assert_eq!(cache.sub_caches.len(), 1);
        assert_eq!(cache.sub_caches["path"].sub_caches.len(), 1);
        assert_eq!(cache.sub_caches["path"].sub_caches["to"].sub_caches.len(), 1);
        assert_eq!(cache.sub_caches["path"].sub_caches["to"].entries.len(), 2);
        assert_eq!(cache.sub_caches["path"].sub_caches["to"].entries["entry"], "me!");
        assert_eq!(cache.sub_caches["path"].sub_caches["to"].entries["other_entry"], "me too!");
        assert_eq!(cache.sub_caches["path"].sub_caches["to"].sub_caches["entry"].sub_caches.len(), 0);
        assert_eq!(cache.sub_caches["path"].sub_caches["to"].sub_caches["entry"].entries.len(), 1);
        assert_eq!(cache.sub_caches["path"].sub_caches["to"].sub_caches["entry"].entries["1_level_lower"], "I'm lower!");
    }    

    #[test]
    fn test_cache_get() {
        let mut cache = Cache::new();
        cache.insert("the/path/to/secrets/secret1", "I'm a secret".to_string());
        cache.insert("the/path/to/secrets/secret2", "I'm another secret".to_string());
        cache.insert("the/path/to/messages/message1", "I'm a message".to_string());
        cache.insert("the/path/to/messages/message2", "I'm Data".to_string());

        assert_eq!(cache.get("the/path/to/secrets/secret1"), Some(&"I'm a secret".to_string()));
        assert_eq!(cache.get("the/path/to/secrets/secret2"), Some(&"I'm another secret".to_string()));
        assert_eq!(cache.get("the/path/to/messages/message1"), Some(&"I'm a message".to_string()));
        assert_eq!(cache.get("the/path/to/messages/message2"), Some(&"I'm Data".to_string()));
    }

    #[test]
    fn test_cache_remove() {
        let mut cache = Cache::new();
        cache.insert("the/path/to/secrets/secret1", "I'm a secret".to_string());
        cache.insert("the/path/to/secrets/secret2", "I'm another secret".to_string());
        cache.insert("the/path/to/messages/message1", "I'm a message".to_string());
        cache.insert("the/path/to/messages/message2", "I'm Data".to_string());

        assert_eq!(cache.remove("the/path/to/secrets/secret1"), Some("I'm a secret".to_string()));
        assert_eq!(cache.get("the/path/to/secrets/secret1"), None);
    }
}