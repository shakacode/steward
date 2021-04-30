use std::collections::{hash_map, HashMap};

/// Environment data for a [`Cmd`](crate::Cmd).
#[derive(Clone)]
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
    pub fn insert<K: ToString, V: ToString>(mut self, k: K, v: V) -> Self {
        self.0.insert(k.to_string(), v.to_string());
        self
    }

    /// Inserts one entry into container by mutating it.
    pub fn insert_cloned<K: ToString, V: ToString>(&self, k: K, v: V) -> Self {
        let mut cloned = self.0.clone();
        cloned.insert(k.to_string(), v.to_string());
        Self(cloned)
    }

    /// Merges two containers by mutating the receiver.
    pub fn extend(mut self, env: Self) -> Self {
        self.0.extend(env.0);
        self
    }

    /// Merges two containers and returns a new cloned one. Doesn't mutate a receiver.
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

/// Convenience struct for dealing with the `PATH` environment variable.
pub struct PATH;

impl PATH {
    #[cfg(unix)]
    const DEL: char = ':';

    #[cfg(windows)]
    const DEL: char = ';';

    /// Gets the `PATH` value from an environment of the current process.
    pub fn get() -> Option<String> {
        Env::parent().get("PATH").map(|x| x.to_owned())
    }

    /// Extends the `PATH` value taken the current process and returns the extended value. It doesn't extend the `PATH` of the current process.
    pub fn extend(x: impl ToString) -> String {
        match PATH::get() {
            Some(path) => format!("{}{}{}", path, PATH::DEL, x.to_string()),
            None => x.to_string(),
        }
    }
}
