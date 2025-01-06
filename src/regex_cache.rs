extern crate lazy_static;
use rand::prelude::*;
use std::{collections::HashMap, sync::Mutex};

use lazy_static::lazy_static;
use regex::Regex;

pub struct RegexCache {
    pub cache: Mutex<HashMap<String, Option<Regex>>>,
    pub capacity: usize,
}

lazy_static! {
    pub static ref REGEX_CACHE: RegexCache = RegexCache::new(100);
}

impl RegexCache {
    pub fn new(capacity: usize) -> Self {
        RegexCache {
            cache: Mutex::new(HashMap::new()),
            capacity,
        }
    }

    pub fn get_regex(&self, pattern: &str) -> Option<Regex> {
        let mut cache = self.cache.lock().unwrap();
        if let Some(re) = cache.get(pattern) {
            return re.clone();
        }
        let re = Regex::new(pattern).ok();
        cache.insert(pattern.to_string(), re.clone());
        re
    }

    pub fn matches(&self, pattern: &str, text: &str) -> bool {
        self.gc();
        let mut cache = self.cache.lock().unwrap();
        if let Some(re) = cache.get(pattern) {
            if let Some(re) = re {
                return re.is_match(text);
            }
            return false;
        }
        let re = Regex::new(pattern).ok();
        cache.insert(pattern.to_string(), re.clone());
        if let Some(re) = re {
            return re.is_match(text);
        } else {
            return false;
        }
    }

    pub fn gc(&self) {
        let mut cache = self.cache.lock().unwrap();
        let mut rng = thread_rng();
        while cache.len() > self.capacity {
            // random remove one
            let keys: Vec<String> = cache.keys().cloned().collect();
            let random_index = rng.gen_range(0..keys.len());

            cache.remove(&keys[random_index]);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_regex_cache() {
        let cache = RegexCache::new(10);
        let re = cache.get_regex(r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}");
        assert_eq!(
            re.unwrap().as_str(),
            r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}"
        );

        assert!(cache.matches(
            r"^\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2}",
            "2021-07-01 12:34:56"
        ));
    }
}
