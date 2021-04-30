use std::{
    fmt,
    path::{Path, PathBuf},
};

use steward::Location;

// Loc holds absolute path to either application directory or file
#[derive(Clone, Debug)]
pub struct Loc(PathBuf);

lazy_static! {
    // We want to find root once per execution and then reuse it
    static ref ROOT: Loc = Loc::find_root();
}

impl Loc {
    pub fn root() -> Self {
        ROOT.to_owned()
    }

    pub fn env_file() -> Self {
        ROOT.join(".env")
    }

    pub fn client() -> Self {
        ROOT.join("client")
    }

    pub fn client_node_modules() -> Self {
        Loc::client().join("node_modules")
    }

    pub fn client_node_modules_bin() -> Self {
        Loc::client_node_modules().join(".bin")
    }
}

impl Loc {
    // Using `Cargo.lock` as a marker of a root directory of the project
    // If user runs a cli from subdirectory, we traverse up the directory tree
    // until `Cargo.lock` is found so all paths defined above still resolved correctly.
    const ROOT_MARKER: &'static str = "Cargo.lock";

    fn find_root() -> Self {
        let cwd = std::env::current_dir().expect("Failed to get current directory of the process");
        Self(Self::traverse(cwd))
    }

    fn traverse(dir: PathBuf) -> PathBuf {
        if dir.join(Self::ROOT_MARKER).exists() {
            dir
        } else {
            Self::traverse(
                dir.parent()
                    .expect("Failed to get parent directory during root search")
                    .to_path_buf(),
            )
        }
    }

    fn join<P: AsRef<Path>>(&self, path: P) -> Self {
        Self(self.as_path().join(path))
    }

    pub fn path(&self) -> &PathBuf {
        &self.0
    }
}

impl Location for Loc {
    fn apex() -> Self {
        Self::root()
    }

    fn as_path(&self) -> &PathBuf {
        self.path()
    }
}

impl AsRef<Path> for Loc {
    fn as_ref(&self) -> &Path {
        &self.0
    }
}

impl fmt::Display for Loc {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_path().to_str().unwrap())
    }
}
