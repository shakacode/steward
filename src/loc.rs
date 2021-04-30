use std::path::PathBuf;

/// Contains a location of file or directory of a project.
///
/// It must be implemented by application since it is project specific.
/// See [example implementation](https://github.com/alexfedoseev/steward/tree/master/examples/cli/loc.rs) in the repository.
pub trait Location: Sized {
    /// Returns a location of the root directory of a project.
    ///
    /// It'd be better to call it `root` but this name left vacant because it's used often
    /// and it's annoying bringing this trait into a scope everytime `Loc::root()`
    /// needs to be called. This way, users can have `root` on their implementation and call it
    /// by just importing their type.
    fn apex() -> Self;

    /// Returns a pointer to an inner path.
    fn as_path(&self) -> &PathBuf;

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
