use std::path::Path;

const EXTENSION_FORMATS: &[(&str, &str)] = &[
    ("c", "c"),
    ("h", "c"),
    ("cc", "cpp"),
    ("cpp", "cpp"),
    ("cxx", "cpp"),
    ("hpp", "cpp"),
    ("css", "css"),
    ("go", "go"),
    ("java", "java"),
    ("js", "javascript"),
    ("cjs", "javascript"),
    ("mjs", "javascript"),
    ("jsx", "jsx"),
    ("json", "json"),
    ("kt", "kotlin"),
    ("kts", "kotlin"),
    ("md", "markdown"),
    ("php", "php"),
    ("py", "python"),
    ("rb", "ruby"),
    ("rs", "rust"),
    ("sh", "bash"),
    ("bash", "bash"),
    ("sql", "sql"),
    ("ts", "typescript"),
    ("tsx", "tsx"),
    ("vue", "vue"),
    ("yaml", "yaml"),
    ("yml", "yaml"),
    ("zig", "zig"),
];

const NAME_FORMATS: &[(&str, &str)] = &[
    ("Dockerfile", "docker"),
    ("GNUmakefile", "makefile"),
    ("Makefile", "makefile"),
];

pub fn format_for_path(path: &Path) -> Option<&'static str> {
    let file_name = path.file_name()?.to_string_lossy();
    for (name, format) in NAME_FORMATS {
        if file_name == *name {
            return Some(format);
        }
    }

    let ext = path.extension()?.to_string_lossy().to_ascii_lowercase();
    EXTENSION_FORMATS
        .iter()
        .find_map(|(candidate, format)| (*candidate == ext).then_some(*format))
}

pub fn supported_formats() -> Vec<&'static str> {
    let mut formats = EXTENSION_FORMATS
        .iter()
        .map(|(_, format)| *format)
        .chain(NAME_FORMATS.iter().map(|(_, format)| *format))
        .collect::<Vec<_>>();
    formats.sort_unstable();
    formats.dedup();
    formats
}
