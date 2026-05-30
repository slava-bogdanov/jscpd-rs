use std::collections::HashMap;
use std::path::Path;
use std::process::Command;
use std::sync::OnceLock;

use regex::Regex;

use crate::detector::{BlamedLine, BlamedLines, DetectionResult, Fragment};

pub fn apply_blame(result: &mut DetectionResult) {
    let mut cache = HashMap::<String, Option<BlamedLines>>::new();
    for clone in &mut result.clones {
        apply_fragment_blame(&mut clone.duplication_a, &mut cache);
        apply_fragment_blame(&mut clone.duplication_b, &mut cache);
    }
}

fn apply_fragment_blame(fragment: &mut Fragment, cache: &mut HashMap<String, Option<BlamedLines>>) {
    let blamed_file = cache
        .entry(fragment.source_id.clone())
        .or_insert_with(|| blame_file(&fragment.source_id));
    fragment.blame = blamed_file
        .as_ref()
        .map(|blame| slice_blame(blame, fragment.start.line, fragment.end.line))
        .filter(|blame| !blame.is_empty());
}

fn blame_file(path: &str) -> Option<BlamedLines> {
    let path = Path::new(path);
    let parent = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let file_name = path.file_name()?;
    let output = Command::new("git")
        .arg("-C")
        .arg(parent)
        .arg("blame")
        .arg("-w")
        .arg("--")
        .arg(file_name)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let stdout = String::from_utf8(output.stdout).ok()?;
    let blamed = parse_git_blame(&stdout);
    (!blamed.is_empty()).then_some(blamed)
}

fn slice_blame(blame: &BlamedLines, start: usize, end: usize) -> BlamedLines {
    (start..=end)
        .filter_map(|line| {
            let key = line.to_string();
            blame.get(&key).cloned().map(|blamed| (key, blamed))
        })
        .collect()
}

fn parse_git_blame(output: &str) -> BlamedLines {
    output
        .lines()
        .filter_map(parse_git_blame_line)
        .map(|line| (line.line.clone(), line))
        .collect()
}

fn parse_git_blame_line(raw_line: &str) -> Option<BlamedLine> {
    let captures = blame_line_regex().captures(raw_line)?;
    let line = captures.get(4)?.as_str().to_string();
    if line.is_empty() {
        return None;
    }

    Some(BlamedLine {
        rev: captures.get(1)?.as_str().to_string(),
        author: captures.get(2)?.as_str().to_string(),
        date: captures.get(3)?.as_str().to_string(),
        line,
    })
}

fn blame_line_regex() -> &'static Regex {
    static REGEX: OnceLock<Regex> = OnceLock::new();
    REGEX.get_or_init(|| {
        Regex::new(
            r"^(.+)\s+\((.+)\s+(\d{4}-\d{2}-\d{2} \d{2}:\d{2}:\d{2} [+-]\d{4})\s+(\d+)\)(.*)$",
        )
        .expect("valid git blame regex")
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_git_blame_lines() {
        let output = "\
ca40bf24 tests/fixtures/file_4.js (Andrey Kucherenko 2013-06-02 23:31:50 +0300 56) footprints = typeof yeti !== \"undefined\";
bbbbbbbb (Bob Smith 2024-01-02 03:04:05 -0700 57) second
";

        let blame = parse_git_blame(output);

        assert_eq!(blame["56"].author, "Andrey Kucherenko");
        assert_eq!(blame["56"].rev, "ca40bf24 tests/fixtures/file_4.js");
        assert_eq!(blame["56"].date, "2013-06-02 23:31:50 +0300");
        assert_eq!(blame["56"].line, "56");
        assert_eq!(blame["57"].author, "Bob Smith");
    }

    #[test]
    fn slices_blame_to_fragment_range() {
        let blame = parse_git_blame(
            "\
aaaaaaaa (Alice 2024-01-01 00:00:00 +0000 1) first
bbbbbbbb (Bob 2024-01-02 00:00:00 +0000 2) second
cccccccc (Carol 2024-01-03 00:00:00 +0000 3) third
",
        );

        let sliced = slice_blame(&blame, 2, 3);

        assert_eq!(sliced.keys().cloned().collect::<Vec<_>>(), vec!["2", "3"]);
        assert_eq!(sliced["2"].author, "Bob");
        assert_eq!(sliced["3"].author, "Carol");
    }
}
