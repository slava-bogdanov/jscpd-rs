mod discovery;
mod gitignore;
mod paths;
mod shebang;

#[cfg(test)]
mod tests;

pub use discovery::discover;

#[derive(Clone, Debug)]
pub struct SourceFile {
    pub source_id: String,
    pub format: String,
    pub content: String,
}
