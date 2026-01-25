use std::path::{Path, PathBuf};

/// A location of file or directory of a project.
///
/// It must be implemented by application since it is project specific.
/// See [example implementation](https://github.com/alexfedoseev/steward/tree/master/examples/cli/loc.rs) in the repository.
pub trait Location: Sized + Send + Sync {
    /// Returns a location of the root directory of a project.
    ///
    /// It'd be better to call it `root` but this name left vacant because it's used often
    /// and it's annoying bringing this trait into a scope everytime `Loc::root()`
    /// needs to be called. This way, users can have `root` on their implementation and call it
    /// by just importing their type.
    fn apex() -> Self;

    /// Returns a pointer to an inner path.
    fn as_path(&self) -> &PathBuf;

    /// Joins a path to the current location.
    fn join<P: AsRef<Path>>(&self, path: P) -> Self;

    /// Formats a path as a relative to the root directory for printing to console.
    fn display(&self) -> String {
        let apex = Self::apex();
        let path = match apex.as_path().parent() {
            None => self.as_path(),
            Some(parent) => {
                let path = self.as_path();
                path.strip_prefix(parent).unwrap_or(path)
            }
        };
        path.display().to_string()
    }
}

/// Generates location functions for a project directory structure.
///
/// This macro creates public functions that return `Loc` instances for each path
/// in your project. It requires a `ROOT` static and a `Loc` type to be defined
/// in the same scope.
///
/// # Syntax
///
/// ```ignore
/// loc! {
///     src,                          // fn src() -> Loc
///     ".env": env_file,             // fn env_file() -> Loc for ".env"
///     src => {                      // fn src() + nested children
///         lib,                      // fn lib() -> Loc for "src/lib"
///         bin => { ... },           // fn bin() + further nesting
///     },
///     apps: web_apps => { ... },    // fn web_apps() + nested children
///     target: _ => { debug },       // skip parent function, process children only
/// }
/// ```
///
/// # Requirements
///
/// The macro expects the following to be defined in scope:
/// - `ROOT` - A static that dereferences to `Loc`
/// - `Loc` - A type that implements [`Location`] trait
///
/// # Examples
///
/// Basic usage with flat structure:
///
/// ```ignore
/// loc! {
///     src,           // fn src() -> Loc { ROOT.join("src") }
///     target,        // fn target() -> Loc { ROOT.join("target") }
///     "Cargo.toml",  // fn Cargo.toml is invalid, use renaming:
/// }
/// ```
///
/// Renaming paths to valid function names:
///
/// ```ignore
/// loc! {
///     "Cargo.toml": cargo_toml,  // fn cargo_toml() -> Loc { ROOT.join("Cargo.toml") }
///     ".env": env_file,          // fn env_file() -> Loc { ROOT.join(".env") }
/// }
/// ```
///
/// Nested directory structure:
///
/// ```ignore
/// loc! {
///     src => {
///         lib,       // fn lib() -> Loc { ROOT.join("src/lib") }
///         bin,       // fn bin() -> Loc { ROOT.join("src/bin") }
///     },
///     tests => {
///         unit,      // fn unit() -> Loc { ROOT.join("tests/unit") }
///         integration,
///     },
/// }
/// ```
///
/// Skipping function generation with `_` (useful for intermediate directories):
///
/// ```ignore
/// loc! {
///     node_modules: _ => {        // No function for node_modules itself
///         ".bin": node_bin,       // fn node_bin() -> Loc { ROOT.join("node_modules/.bin") }
///     },
/// }
/// ```
///
/// Complete example:
///
/// ```ignore
/// use std::{
///     env,
///     path::{Path, PathBuf},
///     sync::LazyLock,
/// };
///
/// use steward::Location;
///
/// #[derive(Clone, Debug)]
/// pub struct Loc(PathBuf);
///
/// static ROOT: LazyLock<Loc> = LazyLock::new(Loc::find_root);
///
/// loc! {
///     src => {
///         main,
///         lib,
///     },
///     "Cargo.toml": cargo_toml,
///     ".gitignore": gitignore,
///     target: _ => {
///         debug,
///         release,
///     },
/// }
///
/// impl Loc {
///     fn find_root() -> Loc {
///         const ROOT_MARKER: &str = "Cargo.lock";
///
///         let cwd = env::current_dir().expect("Failed to get cwd");
///
///         fn traverse(dir: PathBuf) -> Loc {
///             if dir.join(ROOT_MARKER).exists() {
///                 Loc(dir)
///             } else {
///                 traverse(
///                     dir.parent()
///                         .expect("Failed to find root marker")
///                         .to_path_buf(),
///                 )
///             }
///         }
///
///         traverse(cwd)
///     }
/// }
///
/// impl Location for Loc {
///     fn apex() -> Self { ROOT.clone() }
///     fn as_path(&self) -> &PathBuf { &self.0 }
///     fn join<P: AsRef<Path>>(&self, path: P) -> Self { Self(self.0.join(path)) }
/// }
///
/// // Generated functions:
/// // fn src() -> Loc         // ROOT.join("src")
/// // fn main() -> Loc        // ROOT.join("src/main")
/// // fn lib() -> Loc         // ROOT.join("src/lib")
/// // fn cargo_toml() -> Loc  // ROOT.join("Cargo.toml")
/// // fn gitignore() -> Loc   // ROOT.join(".gitignore")
/// // fn debug() -> Loc       // ROOT.join("target/debug")
/// // fn release() -> Loc     // ROOT.join("target/release")
/// // Note: no target() function due to `: _`
/// ```
#[macro_export]
macro_rules! loc {
    // Entry point for root-level paths with or without subpaths
    ( $( $path:tt $( : $name:tt )? $( => { $( $subpath:tt )* } )? ),* $(,)? ) => {
        $(
            loc!(@maybe_node $path, $($name)?);
            $(
                loc!(@subpaths loc!(@path_str $path), $($subpath)*);
            )?
        )*
    };

    // Process subpaths
    (@subpaths $parent_path:expr, $( $child_path:tt $( : $child_name:tt )? $( => { $($grandchild:tt)* } )? ),* $(,)? ) => {
        $(
            loc!(@maybe_subnode $parent_path, $child_path, $($child_name)?);
            $(
                loc!(@subpaths
                    format!("{}{}{}", $parent_path, std::path::MAIN_SEPARATOR, loc!(@path_str $child_path)),
                    $($grandchild)*
                );
            )?
        )*
    };

    // Generate a root-level node
    (@maybe_node $path:tt, ) => {
        pub fn $path() -> Loc {
            ROOT.join(loc!(@path_str $path))
        }
    };
    (@maybe_node $path:tt, $name:tt) => {
        loc!(@generate_if_not_ignored $name, ROOT.join(loc!(@path_str $path)));
    };

    // Generate a subnode
    (@maybe_subnode $parent_path:expr, $path:tt, ) => {
        pub fn $path() -> Loc {
            ROOT.join(format!("{}{}{}",
                $parent_path,
                std::path::MAIN_SEPARATOR,
                loc!(@path_str $path)
            ))
        }
    };
    (@maybe_subnode $parent_path:expr, $path:tt, $name:tt) => {
        loc!(@generate_if_not_ignored $name, ROOT.join(format!("{}{}{}",
            $parent_path,
            std::path::MAIN_SEPARATOR,
            loc!(@path_str $path)
        )));
    };

    // Only generate function if name is not _
    (@generate_if_not_ignored _, $path:expr) => {};
    (@generate_if_not_ignored $name:tt, $path:expr) => {
        pub fn $name() -> Loc {
            $path
        }
    };

    // Handle path strings - converts literals and identifiers to strings
    (@path_str $path:literal) => { $path };
    (@path_str $path:ident) => { stringify!($path) };
}
