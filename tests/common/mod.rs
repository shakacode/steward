use std::path::{Path, PathBuf};
use steward::Location;

#[derive(Clone, Debug)]
pub struct Loc(pub PathBuf);

impl Loc {
    pub fn root() -> Self {
        Self(std::env::current_dir().unwrap())
    }
}

impl Location for Loc {
    fn apex() -> Self {
        Self::root()
    }
    fn as_path(&self) -> &PathBuf {
        &self.0
    }
    fn join<P: AsRef<Path>>(&self, path: P) -> Self {
        Self(self.0.join(path))
    }
}
