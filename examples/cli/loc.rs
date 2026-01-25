use std::{
    env, fmt,
    path::{Path, PathBuf},
    sync::LazyLock,
};

use steward::Location;

// Loc holds absolute path to either application directory or file
#[derive(Clone, Debug)]
pub struct Loc(PathBuf);

static ROOT: LazyLock<Loc> = LazyLock::new(Loc::find_root);

loc! {
    ".env": env_file,
    client => {
        node_modules: client_node_modules => {
            ".bin": client_node_modules_bin,
        },
    },
}

impl Loc {
    pub fn root() -> Self {
        ROOT.clone()
    }

    fn find_root() -> Self {
        const ROOT_MARKER: &str = "Cargo.lock";

        let cwd = env::current_dir().expect("Failed to get current directory of the process");

        fn traverse(dir: PathBuf) -> Loc {
            if dir.join(ROOT_MARKER).exists() {
                Loc(dir)
            } else {
                traverse(
                    dir.parent()
                        .expect("Failed to get parent directory during root search")
                        .to_path_buf(),
                )
            }
        }

        traverse(cwd)
    }

    pub fn join<P: AsRef<Path>>(&self, path: P) -> Self {
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

