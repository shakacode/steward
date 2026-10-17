use std::collections::{hash_map, HashMap};

/// Environment data for a [`Cmd`](crate::Cmd).
#[derive(Clone, Debug)]
pub struct Env(HashMap<String, String>);

impl Env {
    /// Constructs a new container from a [`HashMap`](HashMap).
    pub fn new(data: HashMap<String, String>) -> Self {
        Self(data)
    }

    /// Constructs a new empty container.
    pub fn empty() -> Self {
        Self(HashMap::new())
    }

    /// Constructs a new container from a [`Vec`](Vec).
    pub fn from_vec<K: ToString, V: ToString>(kvs: Vec<(K, V)>) -> Self {
        let mut data = HashMap::with_capacity(kvs.len());
        for (k, v) in kvs {
            data.insert(k.to_string(), v.to_string());
        }
        Self(data)
    }

    /// Constructs a new container with one entry.
    ///
    /// ```
    /// # use steward::Env;
    /// let env = Env::one("KEY", "VALUE");
    /// assert_eq!(env.get("KEY").map(|s| s.as_str()), Some("VALUE"));
    /// ```
    pub fn one<K: ToString, V: ToString>(k: K, v: V) -> Self {
        let mut data = HashMap::with_capacity(1);
        data.insert(k.to_string(), v.to_string());
        Self(data)
    }

    /// Constructs a new container with data from an environment of the current process.
    pub fn parent() -> Self {
        let env = std::env::vars();
        let mut data = HashMap::new();
        for (k, v) in env {
            data.insert(k, v);
        }
        Self(data)
    }

    /// Inserts one entry into existing container by mutating it.
    #[must_use]
    pub fn insert<K: ToString, V: ToString>(mut self, k: K, v: V) -> Self {
        self.0.insert(k.to_string(), v.to_string());
        self
    }

    /// Inserts one entry into container by mutating it.
    #[must_use]
    pub fn insert_cloned<K: ToString, V: ToString>(&self, k: K, v: V) -> Self {
        let mut cloned = self.0.clone();
        cloned.insert(k.to_string(), v.to_string());
        Self(cloned)
    }

    /// Merges two containers by mutating the receiver.
    #[must_use]
    pub fn extend(mut self, env: Self) -> Self {
        self.0.extend(env.0);
        self
    }

    /// Merges two containers and returns a new cloned one. Doesn't mutate a receiver.
    #[must_use]
    pub fn extend_cloned(&self, env: Self) -> Self {
        Self(self.0.clone().into_iter().chain(env.0).collect())
    }

    /// Retrives a value from a container by the provided key.
    pub fn get(&self, k: &str) -> Option<&String> {
        self.0.get(k)
    }
}

impl IntoIterator for Env {
    type Item = (String, String);
    type IntoIter = hash_map::IntoIter<String, String>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.into_iter()
    }
}

impl<'a> IntoIterator for &'a Env {
    type Item = (&'a str, &'a str);
    type IntoIter = std::iter::Map<
        hash_map::Iter<'a, String, String>,
        fn((&'a String, &'a String)) -> (&'a str, &'a str),
    >;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter().map(|(k, v)| (k.as_str(), v.as_str()))
    }
}

/// Convenience struct for dealing with the `PATH` environment variable.
pub struct PATH;

impl PATH {
    #[cfg(unix)]
    const DEL: char = ':';

    #[cfg(windows)]
    const DEL: char = ';';

    /// Gets the `PATH` value from an environment of the current process.
    #[cfg(unix)]
    pub fn get() -> Option<String> {
        Env::parent().get("PATH").map(|x| x.to_owned())
    }

    /// Gets the `PATH` value from an environment of the current process.
    #[cfg(windows)]
    pub fn get() -> Option<String> {
        std::env::var("PATH").ok()
    }

    /// Extends the `PATH` value taken the current process and returns the extended value. It doesn't extend the `PATH` of the current process.
    pub fn extend(x: impl ToString) -> String {
        match PATH::get() {
            Some(path) => format!("{}{}{}", path, PATH::DEL, x.to_string()),
            None => x.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    #[test]
    fn env_empty() {
        let env = Env::empty();
        assert!(env.get("anything").is_none());
        assert_eq!(env.into_iter().count(), 0);
    }

    #[test]
    fn env_new_from_hashmap() {
        let mut map = HashMap::new();
        map.insert("KEY".to_string(), "VALUE".to_string());
        let env = Env::new(map);
        assert_eq!(env.get("KEY").map(|s| s.as_str()), Some("VALUE"));
    }

    #[test]
    fn env_from_vec() {
        let env = Env::from_vec(vec![("A", "1"), ("B", "2")]);
        assert_eq!(env.get("A").map(|s| s.as_str()), Some("1"));
        assert_eq!(env.get("B").map(|s| s.as_str()), Some("2"));
        assert_eq!(env.into_iter().count(), 2);
    }

    #[test]
    fn env_one() {
        let env = Env::one("KEY", "VALUE");
        assert_eq!(env.get("KEY").map(|s| s.as_str()), Some("VALUE"));
        assert_eq!(env.into_iter().count(), 1);
    }

    #[test]
    fn env_parent_matches_current_process_env() {
        let env = Env::parent();
        let actual: std::collections::HashMap<String, String> = std::env::vars().collect();
        assert_eq!((&env).into_iter().count(), actual.len());
        for (k, v) in &actual {
            assert_eq!(
                env.get(k).map(|s| s.as_str()),
                Some(v.as_str()),
                "mismatch on key {}",
                k
            );
        }
    }

    #[test]
    fn env_insert() {
        let env = Env::empty().insert("A", "1").insert("B", "2");
        assert_eq!(env.get("A").map(|s| s.as_str()), Some("1"));
        assert_eq!(env.get("B").map(|s| s.as_str()), Some("2"));
    }

    #[test]
    fn env_insert_cloned_does_not_mutate_original() {
        let env = Env::one("A", "1");
        let extended = env.insert_cloned("B", "2");
        assert!(env.get("B").is_none());
        assert_eq!(extended.get("A").map(|s| s.as_str()), Some("1"));
        assert_eq!(extended.get("B").map(|s| s.as_str()), Some("2"));
    }

    #[test]
    fn env_extend() {
        let a = Env::one("A", "1");
        let b = Env::one("B", "2");
        let merged = a.extend(b);
        assert_eq!(merged.get("A").map(|s| s.as_str()), Some("1"));
        assert_eq!(merged.get("B").map(|s| s.as_str()), Some("2"));
    }

    #[test]
    fn env_extend_cloned_does_not_mutate_original() {
        let a = Env::one("A", "1");
        let b = Env::one("B", "2");
        let merged = a.extend_cloned(b);
        assert!(a.get("B").is_none());
        assert_eq!(merged.get("A").map(|s| s.as_str()), Some("1"));
        assert_eq!(merged.get("B").map(|s| s.as_str()), Some("2"));
    }

    #[test]
    fn env_into_iterator_owned() {
        let env = Env::from_vec(vec![("A", "1"), ("B", "2")]);
        let mut items: Vec<(String, String)> = env.into_iter().collect();
        items.sort_by(|a, b| a.0.cmp(&b.0));
        assert_eq!(
            items,
            vec![
                ("A".to_string(), "1".to_string()),
                ("B".to_string(), "2".to_string())
            ]
        );
    }

    #[test]
    fn env_into_iterator_borrowed() {
        let env = Env::from_vec(vec![("A", "1"), ("B", "2")]);
        let mut items: Vec<(&str, &str)> = (&env).into_iter().collect();
        items.sort_by(|a, b| a.0.cmp(b.0));
        assert_eq!(items, vec![("A", "1"), ("B", "2")]);
    }

    #[test]
    fn env_clone_is_independent() {
        let env = Env::one("A", "1");
        let cloned = env.clone().insert("B", "2");
        assert!(env.get("B").is_none());
        assert_eq!(cloned.get("B").map(|s| s.as_str()), Some("2"));
    }

    #[test]
    fn path_get_returns_some() {
        assert!(PATH::get().is_some());
    }

    #[test]
    fn path_extend_appends_to_existing() {
        let extended = PATH::extend("/custom/bin");
        assert!(extended.contains("/custom/bin"));
        assert!(extended.len() > "/custom/bin".len());
    }
}
