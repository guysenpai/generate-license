//! Pure logic: extract identifiers, render texts, plan the writes.

use std::path::{Path, PathBuf};

use registry::{License, Text};

pub mod registry;

/// Why the tool stops. The variant decides the exit code.
#[derive(Debug, PartialEq)]
pub enum Error {
    /// Bad invocation: exit code 2.
    Usage(String),
    /// Refusal or runtime failure: exit code 1.
    Failure(String),
}

/// The licenses named by an expression, one per distinct text.
#[derive(Default)]
pub struct Extraction {
    pub licenses: Vec<&'static License>,
    pub warnings: Vec<String>,
}

/// Finds the licenses named in `expression`. There is no expression tree:
/// `AND` and `OR` are skipped and the SPDX grammar is not validated.
pub fn extract(expression: &str) -> Result<Extraction, Error> {
    let is = |token: &str, word: &str| token.eq_ignore_ascii_case(word);
    let tokens: Vec<&str> = expression
        .split(|c: char| c.is_whitespace() || matches!(c, '(' | ')' | '/'))
        .filter(|t| !t.is_empty() && !is(t, "AND") && !is(t, "OR"))
        .collect();
    if tokens.iter().any(|t| is(t, "WITH")) {
        let message = "license exceptions (WITH) are not supported";
        return Err(Error::Failure(message.into()));
    }

    let mut extraction = Extraction::default();
    for token in tokens {
        let (license, warning) = resolve(token)?;
        let same_text = |l: &&License| l.text.file == license.text.file;
        if !extraction.licenses.iter().any(same_text) {
            extraction.licenses.push(license);
        }
        extraction
            .warnings
            .extend(warning.filter(|w| !extraction.warnings.contains(w)));
    }
    if extraction.licenses.is_empty() {
        let message = "missing license identifier (try --list)";
        return Err(Error::Usage(message.into()));
    }
    Ok(extraction)
}

/// Resolves one token to a license and, for a deprecated identifier, a warning.
fn resolve(token: &str) -> Result<(&'static License, Option<String>), Error> {
    let (base, plus) = match token.strip_suffix('+') {
        Some(base) => (base, "+"),
        None => (token, ""),
    };
    if let Some(alias) = registry::deprecated(base) {
        let successor = if plus.is_empty() { "only" } else { "or-later" };
        let license = registry::find(&format!("{alias}-{successor}"))
            .expect("deprecated identifiers have both successors");
        let warning = format!(
            "'{alias}{plus}' is a deprecated SPDX identifier, using '{}'",
            license.id
        );
        return Ok((license, Some(warning)));
    }
    let unknown = || {
        let tip = tip(base).map(|t| format!("tip: {t}\n")).unwrap_or_default();
        Error::Failure(format!(
            "unknown or unsupported license '{token}'\n{tip}\
             Run 'generate-license --list' to see supported licenses."
        ))
    };
    registry::find(base).map(|l| (l, None)).ok_or_else(unknown)
}

/// Suggests identifiers for an unknown one. Prefix matches come first:
/// otherwise `BSD` would suggest `0BSD` instead of the two BSD licenses.
fn tip(input: &str) -> Option<String> {
    let input = input.to_ascii_lowercase();
    let ids = || registry::LICENSES.iter().map(|l| l.id);
    let starts_with_input =
        |id: &&str| !input.is_empty() && id.to_ascii_lowercase().starts_with(&input);
    let prefixed: Vec<&str> = ids().filter(starts_with_input).collect();
    match prefixed[..] {
        [] => ids()
            .map(|id| (levenshtein(&input, &id.to_ascii_lowercase()), id))
            .filter(|(distance, _)| *distance <= 2)
            .min_by_key(|(distance, _)| *distance)
            .map(|(_, id)| format!("did you mean '{id}'?")),
        [id] => Some(format!("did you mean '{id}'?")),
        _ => Some(format!("did you mean one of: {}?", prefixed.join(", "))),
    }
}

fn levenshtein(a: &str, b: &str) -> usize {
    let b: Vec<char> = b.chars().collect();
    // Distances from the current prefix of `a` to every prefix of `b`.
    let mut row: Vec<usize> = (0..=b.len()).collect();
    for (i, char_a) in a.chars().enumerate() {
        let mut diagonal = row[0];
        row[0] = i + 1;
        for (j, char_b) in b.iter().enumerate() {
            let substitution = diagonal + usize::from(char_a != *char_b);
            diagonal = row[j + 1];
            row[j + 1] = substitution.min(diagonal + 1).min(row[j] + 1);
        }
    }
    row[b.len()]
}

const YEAR: &str = "[year]";
const HOLDER: &str = "[fullname]";

/// Whether `text` has a copyright line to fill.
pub fn needs_copyright(text: &str) -> bool {
    text.contains(YEAR) || text.contains(HOLDER)
}

/// Fills the copyright line of `text`. `year: None` leaves the year out.
/// Single pass: a holder containing `[year]` is not substituted again.
pub fn render(text: &str, year: Option<&str>, holder: &str) -> String {
    let mut out = String::with_capacity(text.len() + holder.len());
    let mut rest = text;
    while let Some(bracket) = rest.find('[') {
        out.push_str(&rest[..bracket]);
        rest = &rest[bracket..];
        if let Some(after) = rest.strip_prefix(YEAR) {
            rest = match year {
                Some(year) => {
                    out.push_str(year);
                    after
                }
                None => {
                    let after = after.strip_prefix(',').unwrap_or(after);
                    after.strip_prefix(' ').unwrap_or(after)
                }
            };
        } else if let Some(after) = rest.strip_prefix(HOLDER) {
            out.push_str(holder);
            rest = after;
        } else {
            out.push('[');
            rest = &rest[1..];
        }
    }
    out + rest
}

/// Civil year (UTC) of a number of seconds since the Unix epoch.
pub fn year_of(unix_seconds: u64) -> u64 {
    let mut days = unix_seconds / 86_400;
    let mut year = 1970;
    loop {
        let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
        let length = if leap { 366 } else { 365 };
        if days < length {
            return year;
        }
        days -= length;
        year += 1;
    }
}

/// Names the files to write for `licenses` (already one per text), in order.
/// `output` replaces the name and is only valid for a single file.
pub fn name_files(
    licenses: &[&'static License],
    output: Option<&Path>,
) -> Result<Vec<(PathBuf, &'static Text)>, Error> {
    let companions: Vec<&Text> = licenses.iter().filter_map(|l| l.text.companion()).collect();
    let mut files: Vec<(PathBuf, &Text)> = Vec::new();
    for license in licenses {
        if let Some(companion) = license.text.companion() {
            files.push(("COPYING.LESSER".into(), license.text));
            files.push(("COPYING".into(), companion));
        } else if !companions.iter().any(|c| c.file == license.text.file) {
            files.push((format!("LICENSE-{}", license.suffix()).into(), license.text));
        }
    }
    if let [(name, _)] = &mut files[..] {
        *name = output.unwrap_or(Path::new("LICENSE")).into();
    } else if output.is_some() {
        let names: Vec<String> = files.iter().map(|(n, _)| n.display().to_string()).collect();
        return Err(Error::Usage(format!(
            "--output takes a single file, but {} would be written: {}",
            names.len(),
            names.join(", ")
        )));
    }
    Ok(files)
}

/// Whether a file name of the target directory denotes a license file:
/// `^(un)?licen[sc]e($|[.\-_])` or `^copying($|\.)`, ignoring case.
pub fn is_license_file(name: &str) -> bool {
    let name = name.to_ascii_lowercase();
    let ends_stem =
        |rest: &str, separators: &[char]| rest.is_empty() || rest.starts_with(separators);
    let license = name.strip_prefix("un").unwrap_or(&name);
    let license = license
        .strip_prefix("license")
        .or_else(|| license.strip_prefix("licence"));
    license.is_some_and(|rest| ends_stem(rest, &['.', '-', '_']))
        || name
            .strip_prefix("copying")
            .is_some_and(|rest| ends_stem(rest, &['.']))
}

/// What is already on disk at a target path.
pub struct Existing {
    /// File name the path resolves to: `license` for the target `LICENSE`
    /// on a case-insensitive file system.
    pub name: String,
    pub content: Vec<u8>,
}

/// A file the tool wants on disk.
pub struct Target {
    pub path: PathBuf,
    pub content: String,
    pub existing: Option<Existing>,
}

#[derive(Debug, PartialEq)]
pub enum Step {
    Create,
    Overwrite,
    Unchanged,
}

/// Decides what to do with each target, in order, or refuses everything.
/// `dir_files` are the regular files of the target directory (empty when the
/// output path is explicit: other license files are not checked then).
pub fn plan(targets: &[Target], dir_files: &[String], force: bool) -> Result<Vec<Step>, Error> {
    let step = |target: &Target| match &target.existing {
        None => Ok(Step::Create),
        Some(existing) if existing.content == target.content.as_bytes() => Ok(Step::Unchanged),
        Some(_) if force => Ok(Step::Overwrite),
        Some(_) => Err(Error::Failure(format!(
            "{} already exists and differs (use --force to overwrite)",
            target.path.display()
        ))),
    };
    let steps = targets
        .iter()
        .map(step)
        .collect::<Result<Vec<Step>, Error>>()?;

    let is_target = |name: &str| {
        targets.iter().any(|t| {
            t.path.as_os_str() == name || t.existing.as_ref().is_some_and(|e| e.name == name)
        })
    };
    let mut others: Vec<&str> = dir_files
        .iter()
        .map(String::as_str)
        .filter(|name| is_license_file(name) && !is_target(name))
        .collect();
    others.sort_unstable();
    if !force && !others.is_empty() {
        return Err(Error::Failure(format!(
            "other license files found: {} (remove them, use -o, or pass --force)",
            others.join(", ")
        )));
    }
    Ok(steps)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ids(expression: &str) -> Vec<&'static str> {
        let licenses = extract(expression).unwrap().licenses;
        licenses.iter().map(|l| l.id).collect()
    }

    fn failure(expression: &str) -> String {
        match extract(expression) {
            Err(Error::Failure(message)) => message,
            _ => panic!("expected a failure for {expression:?}"),
        }
    }

    const RUN_LIST: &str = "Run 'generate-license --list' to see supported licenses.";

    #[test]
    fn extracts_identifiers_in_canonical_case() {
        assert_eq!(ids("MIT"), ["MIT"]);
        assert_eq!(ids("mit"), ["MIT"]);
        assert_eq!(ids("apache-2.0"), ["Apache-2.0"]);
    }

    #[test]
    fn splits_on_operators_spaces_parentheses_and_slashes() {
        assert_eq!(ids("MIT OR Apache-2.0"), ["MIT", "Apache-2.0"]);
        assert_eq!(
            ids("(MIT OR Apache-2.0) AND CC-BY-4.0"),
            ["MIT", "Apache-2.0", "CC-BY-4.0"]
        );
        assert_eq!(ids("MIT/Apache-2.0"), ["MIT", "Apache-2.0"]);
        assert_eq!(ids("MIT Apache-2.0"), ["MIT", "Apache-2.0"]);
        assert_eq!(
            ids("mit or apache-2.0 and isc"),
            ["MIT", "Apache-2.0", "ISC"]
        );
        assert_eq!(ids("MIT OR"), ["MIT"]);
    }

    #[test]
    fn keeps_the_first_identifier_of_each_text() {
        assert_eq!(ids("MIT OR MIT"), ["MIT"]);
        assert_eq!(ids("GPL-3.0-only OR GPL-3.0-or-later"), ["GPL-3.0-only"]);
        assert_eq!(
            ids("GPL-3.0-or-later OR GPL-3.0-only"),
            ["GPL-3.0-or-later"]
        );
    }

    #[test]
    fn deprecated_identifiers_resolve_with_a_warning() {
        let only = extract("gpl-3.0").unwrap();
        assert_eq!(only.licenses[0].id, "GPL-3.0-only");
        assert_eq!(
            only.warnings,
            ["'GPL-3.0' is a deprecated SPDX identifier, using 'GPL-3.0-only'"]
        );

        let later = extract("GPL-3.0+").unwrap();
        assert_eq!(later.licenses[0].id, "GPL-3.0-or-later");
        assert_eq!(
            later.warnings,
            ["'GPL-3.0+' is a deprecated SPDX identifier, using 'GPL-3.0-or-later'"]
        );

        assert_eq!(extract("GPL-3.0 OR GPL-3.0").unwrap().warnings.len(), 1);
    }

    #[test]
    fn a_trailing_plus_is_otherwise_ignored() {
        let extraction = extract("Apache-2.0+").unwrap();
        assert_eq!(extraction.licenses[0].id, "Apache-2.0");
        assert!(extraction.warnings.is_empty());
        assert_eq!(ids("GPL-3.0-only+"), ["GPL-3.0-only"]);
    }

    #[test]
    fn exceptions_are_refused_before_anything_is_resolved() {
        let message = "license exceptions (WITH) are not supported";
        assert_eq!(failure("MIT WITH Classpath-exception-2.0"), message);
        assert_eq!(failure("MTI with Classpath-exception-2.0"), message);
    }

    #[test]
    fn an_unknown_identifier_suggests_the_closest_one() {
        assert_eq!(
            failure("MTI"),
            format!("unknown or unsupported license 'MTI'\ntip: did you mean 'MIT'?\n{RUN_LIST}")
        );
        assert_eq!(
            failure("MIT OR Apache2.0"),
            format!(
                "unknown or unsupported license 'Apache2.0'\ntip: did you mean 'Apache-2.0'?\n{RUN_LIST}"
            )
        );
    }

    #[test]
    fn an_unknown_identifier_suggests_those_it_is_a_prefix_of() {
        assert_eq!(
            failure("GPL-3"),
            format!(
                "unknown or unsupported license 'GPL-3'\n\
                 tip: did you mean one of: GPL-3.0-only, GPL-3.0-or-later?\n{RUN_LIST}"
            )
        );
        // The prefix rule wins over the distance rule, which would say 0BSD.
        assert!(failure("bsd").contains("did you mean one of: BSD-2-Clause, BSD-3-Clause?"));
        assert!(failure("Apach").contains("tip: did you mean 'Apache-2.0'?"));
        assert!(failure("GPL-3+").contains("'GPL-3+'\ntip: did you mean one of: GPL-3.0-only,"));
    }

    #[test]
    fn an_unknown_identifier_without_a_close_match_gets_no_tip() {
        let no_tip = |token: &str| format!("unknown or unsupported license '{token}'\n{RUN_LIST}");
        assert_eq!(failure("Proprietary"), no_tip("Proprietary"));
        assert_eq!(failure("+"), no_tip("+"));
        // Distance 3 from MIT and ISC: one too many.
        assert_eq!(failure("XYZ"), no_tip("XYZ"));
    }

    #[test]
    fn an_expression_without_identifier_is_a_usage_error() {
        let usage = Error::Usage("missing license identifier (try --list)".into());
        assert_eq!(
            extract("OR").err(),
            Some(Error::Usage(
                "missing license identifier (try --list)".into()
            ))
        );
        assert_eq!(extract("").err(), Some(usage));
    }

    #[test]
    fn levenshtein_counts_single_character_edits() {
        assert_eq!(levenshtein("", ""), 0);
        assert_eq!(levenshtein("mit", "mit"), 0);
        assert_eq!(levenshtein("", "abc"), 3);
        assert_eq!(levenshtein("abc", ""), 3);
        assert_eq!(levenshtein("mti", "mit"), 2);
        assert_eq!(levenshtein("kitten", "sitting"), 3);
        assert_eq!(levenshtein("flaw", "lawn"), 2);
    }

    const HOLDER: &str = "Ada Lovelace";

    fn copyright_line(id: &str, year: Option<&str>) -> String {
        let body = registry::find(id).unwrap().text.body;
        let source: Vec<&str> = body.lines().collect();
        let rendered = render(body, year, HOLDER);
        let changed: Vec<&str> = rendered
            .lines()
            .zip(&source)
            .filter(|(new, old)| new != *old)
            .map(|(new, _)| new)
            .collect();
        assert_eq!(rendered.lines().count(), source.len(), "{id}");
        assert_eq!(changed.len(), 1, "{id}: only the copyright line changes");
        changed[0].to_string()
    }

    #[test]
    fn renders_the_copyright_line_with_a_year() {
        for (id, line) in [
            ("MIT", "Copyright (c) 2026 Ada Lovelace"),
            ("ISC", "Copyright (c) 2026 Ada Lovelace"),
            ("0BSD", "Copyright (c) 2026 Ada Lovelace"),
            ("MIT-0", "Copyright 2026 Ada Lovelace"),
            ("BSD-2-Clause", "Copyright (c) 2026, Ada Lovelace"),
            ("BSD-3-Clause", "Copyright (c) 2026, Ada Lovelace"),
            ("Zlib", "(C) 2026 Ada Lovelace"),
        ] {
            assert_eq!(copyright_line(id, Some("2026")), line, "{id}");
        }
    }

    #[test]
    fn renders_the_copyright_line_without_a_year() {
        for (id, line) in [
            ("MIT", "Copyright (c) Ada Lovelace"),
            ("ISC", "Copyright (c) Ada Lovelace"),
            ("0BSD", "Copyright (c) Ada Lovelace"),
            ("MIT-0", "Copyright Ada Lovelace"),
            ("BSD-2-Clause", "Copyright (c) Ada Lovelace"),
            ("BSD-3-Clause", "Copyright (c) Ada Lovelace"),
            ("Zlib", "(C) Ada Lovelace"),
        ] {
            assert_eq!(copyright_line(id, None), line, "{id}");
        }
    }

    #[test]
    fn a_year_range_is_free_text() {
        assert_eq!(
            render("(c) [year], [fullname]\n", Some("2019-2026"), HOLDER),
            "(c) 2019-2026, Ada Lovelace\n"
        );
    }

    #[test]
    fn substituted_values_are_not_substituted_again() {
        assert_eq!(
            render("[year] [fullname]", Some("[fullname]"), "[year] [fullname]"),
            "[fullname] [year] [fullname]"
        );
        assert_eq!(render("[year] [fullname]", None, "[year]"), "[year]");
    }

    #[test]
    fn omitting_the_year_removes_only_one_comma_and_one_space() {
        assert_eq!(render("[year],  x", None, HOLDER), " x");
        assert_eq!(render("[year] , x", None, HOLDER), ", x");
        assert_eq!(render("[year]", None, HOLDER), "");
        assert_eq!(render("[year],", None, HOLDER), "");
    }

    #[test]
    fn other_brackets_are_left_alone() {
        let text = "[yyyy] [name of copyright owner] [] [year [fullname\n";
        assert_eq!(render(text, Some("2026"), HOLDER), text);
    }

    #[test]
    fn texts_without_placeholder_render_byte_for_byte() {
        let mut checked = 0;
        for license in &registry::LICENSES {
            let body = license.text.body;
            if !needs_copyright(body) {
                assert!(render(body, Some("2026"), HOLDER) == body, "{}", license.id);
                assert!(render(body, None, HOLDER) == body, "{}", license.id);
                checked += 1;
            }
        }
        // 18 texts, 5 of them reachable through two identifiers.
        assert_eq!(checked, 23);
    }

    #[test]
    fn a_text_needs_copyright_if_it_has_either_placeholder() {
        assert!(needs_copyright("Copyright [year]"));
        assert!(needs_copyright("by [fullname]"));
        assert!(!needs_copyright(
            "Copyright [yyyy] [name of copyright owner]"
        ));
    }

    #[test]
    fn converts_unix_seconds_to_the_utc_year() {
        assert_eq!(year_of(0), 1970);
        assert_eq!(year_of(1_704_067_199), 2023);
        assert_eq!(year_of(1_704_067_200), 2024);
        // 2024-02-29T12:00:00Z and the last second of that leap year.
        assert_eq!(year_of(1_709_208_000), 2024);
        assert_eq!(year_of(1_735_689_599), 2024);
        assert_eq!(year_of(1_735_689_600), 2025);
        // 2100 is not a leap year.
        assert_eq!(year_of(4_102_444_800), 2100);
        assert_eq!(year_of(4_133_980_799), 2100);
        assert_eq!(year_of(4_133_980_800), 2101);
    }

    fn names(expression: &str) -> Vec<String> {
        let licenses = extract(expression).unwrap().licenses;
        let files = name_files(&licenses, None).unwrap();
        files
            .iter()
            .map(|(path, _)| path.display().to_string())
            .collect()
    }

    #[test]
    fn a_single_file_is_named_license() {
        assert_eq!(names("MIT"), ["LICENSE"]);
        assert_eq!(names("GPL-3.0-or-later"), ["LICENSE"]);
        assert_eq!(names("GPL-3.0-only OR GPL-3.0-or-later"), ["LICENSE"]);
        assert_eq!(names("Apache-2.0"), ["LICENSE"]);
    }

    #[test]
    fn several_files_are_named_after_their_identifier() {
        assert_eq!(
            names("MIT OR Apache-2.0"),
            ["LICENSE-MIT", "LICENSE-APACHE"]
        );
        assert_eq!(
            names("MIT AND CC-BY-NC-4.0"),
            ["LICENSE-MIT", "LICENSE-CC-BY-NC-4.0"]
        );
        assert_eq!(
            names("gpl-2.0-or-later OR mit"),
            ["LICENSE-GPL-2.0-or-later", "LICENSE-MIT"]
        );
    }

    #[test]
    fn lgpl_3_is_written_as_copying_lesser_next_to_copying() {
        assert_eq!(names("LGPL-3.0-only"), ["COPYING.LESSER", "COPYING"]);
        assert_eq!(
            names("LGPL-3.0-or-later OR MIT"),
            ["COPYING.LESSER", "COPYING", "LICENSE-MIT"]
        );
        assert_eq!(
            names("MIT OR LGPL-3.0-only"),
            ["LICENSE-MIT", "COPYING.LESSER", "COPYING"]
        );
    }

    #[test]
    fn gpl_3_requested_with_lgpl_3_is_written_once_as_copying() {
        assert_eq!(
            names("LGPL-3.0-only OR GPL-3.0-only"),
            ["COPYING.LESSER", "COPYING"]
        );
        assert_eq!(
            names("GPL-3.0-or-later OR LGPL-3.0-only"),
            ["COPYING.LESSER", "COPYING"]
        );
        assert_eq!(
            names("GPL-3.0-only OR MIT OR LGPL-3.0-only"),
            ["LICENSE-MIT", "COPYING.LESSER", "COPYING"]
        );
    }

    #[test]
    fn named_files_carry_the_matching_text() {
        let licenses = extract("LGPL-3.0-only OR MIT").unwrap().licenses;
        let files = name_files(&licenses, None).unwrap();
        let texts: Vec<&str> = files.iter().map(|(_, text)| text.file).collect();
        assert_eq!(texts, ["lgpl-3.0.txt", "gpl-3.0.txt", "mit.txt"]);
    }

    #[test]
    fn an_output_path_replaces_the_name_of_a_single_file() {
        let licenses = extract("MIT").unwrap().licenses;
        for output in ["docs/NOTICE", "-"] {
            let files = name_files(&licenses, Some(Path::new(output))).unwrap();
            assert_eq!(files.len(), 1);
            assert_eq!(files[0].0, Path::new(output));
            assert_eq!(files[0].1.file, "mit.txt");
        }
    }

    #[test]
    fn an_output_path_with_several_files_is_a_usage_error() {
        for (expression, listed) in [
            ("MIT OR Apache-2.0", "LICENSE-MIT, LICENSE-APACHE"),
            ("LGPL-3.0-only", "COPYING.LESSER, COPYING"),
        ] {
            let licenses = extract(expression).unwrap().licenses;
            assert_eq!(
                name_files(&licenses, Some(Path::new("out"))).err(),
                Some(Error::Usage(format!(
                    "--output takes a single file, but 2 would be written: {listed}"
                )))
            );
        }
    }

    #[test]
    fn recognizes_license_file_names() {
        for name in [
            "LICENSE",
            "license",
            "LICENCE",
            "License.md",
            "LICENSE-MIT",
            "LICENSE_APACHE",
            "licence.txt",
            "UNLICENSE",
            "Unlicense.txt",
            "unlicence-old",
            "COPYING",
            "copying",
            "COPYING.LESSER",
            "copying.txt",
        ] {
            assert!(is_license_file(name), "{name}");
        }
        for name in [
            "LICENSES",
            "LICENSEE.md",
            "LICENS",
            "MIT-LICENSE",
            "my-license.txt",
            "unlicensed",
            "un",
            "COPYING-GPL",
            "COPYING_OLD",
            "COPYINGS",
            "copy",
            "README.md",
            "Cargo.toml",
            "",
        ] {
            assert!(!is_license_file(name), "{name}");
        }
    }

    const TEXT: &str = "MIT License\n";

    fn target(name: &str, existing: Option<(&str, &str)>) -> Target {
        Target {
            path: name.into(),
            content: TEXT.into(),
            existing: existing.map(|(name, content)| Existing {
                name: name.into(),
                content: content.into(),
            }),
        }
    }

    fn files(names: &[&str]) -> Vec<String> {
        names.iter().map(|n| n.to_string()).collect()
    }

    fn refusal(message: &str) -> Result<Vec<Step>, Error> {
        Err(Error::Failure(message.into()))
    }

    const DIFFERS: &str = "LICENSE already exists and differs (use --force to overwrite)";

    #[test]
    fn an_absent_target_is_created() {
        let targets = [target("LICENSE", None)];
        let dir = files(&["README.md", "Cargo.toml", "LICENSES"]);
        assert_eq!(plan(&targets, &dir, false), Ok(vec![Step::Create]));
        assert_eq!(plan(&targets, &dir, true), Ok(vec![Step::Create]));
    }

    #[test]
    fn an_identical_target_is_left_unchanged() {
        let targets = [target("LICENSE", Some(("LICENSE", TEXT)))];
        let dir = files(&["LICENSE"]);
        assert_eq!(plan(&targets, &dir, false), Ok(vec![Step::Unchanged]));
        assert_eq!(plan(&targets, &dir, true), Ok(vec![Step::Unchanged]));
    }

    #[test]
    fn a_different_target_is_refused_unless_forced() {
        let targets = [target("LICENSE", Some(("LICENSE", "MIT License\r\n")))];
        let dir = files(&["LICENSE"]);
        assert_eq!(plan(&targets, &dir, false), refusal(DIFFERS));
        assert_eq!(plan(&targets, &dir, true), Ok(vec![Step::Overwrite]));
    }

    #[test]
    fn other_license_files_are_refused_unless_forced() {
        let targets = [target("LICENSE", None)];
        let dir = files(&["README.md", "LICENSE.md", "COPYING"]);
        assert_eq!(
            plan(&targets, &dir, false),
            refusal(
                "other license files found: COPYING, LICENSE.md \
                 (remove them, use -o, or pass --force)"
            )
        );
        assert_eq!(plan(&targets, &dir, true), Ok(vec![Step::Create]));
    }

    #[test]
    fn other_license_files_are_refused_even_when_targets_are_unchanged() {
        let targets = [target("LICENSE", Some(("LICENSE", TEXT)))];
        let dir = files(&["LICENSE", "UNLICENSE"]);
        assert_eq!(
            plan(&targets, &dir, false),
            refusal("other license files found: UNLICENSE (remove them, use -o, or pass --force)")
        );
    }

    #[test]
    fn a_different_target_is_reported_before_other_files() {
        let targets = [target("LICENSE", Some(("LICENSE", "old")))];
        let dir = files(&["LICENSE", "COPYING"]);
        assert_eq!(plan(&targets, &dir, false), refusal(DIFFERS));
    }

    #[test]
    fn the_file_a_target_resolves_to_is_not_another_license_file() {
        // Case-insensitive file system: `license` is the target `LICENSE`.
        let dir = files(&["license"]);
        let same = [target("LICENSE", Some(("license", TEXT)))];
        assert_eq!(plan(&same, &dir, false), Ok(vec![Step::Unchanged]));
        let different = [target("LICENSE", Some(("license", "old")))];
        assert_eq!(plan(&different, &dir, false), refusal(DIFFERS));

        // Case-sensitive file system: `license` is a different file.
        let absent = [target("LICENSE", None)];
        assert_eq!(
            plan(&absent, &dir, false),
            refusal("other license files found: license (remove them, use -o, or pass --force)")
        );
    }

    #[test]
    fn nothing_is_planned_if_any_target_is_refused() {
        let targets = [
            target("LICENSE-MIT", None),
            target("LICENSE-APACHE", Some(("LICENSE-APACHE", "old"))),
        ];
        let dir = files(&["LICENSE-APACHE"]);
        assert_eq!(
            plan(&targets, &dir, false),
            refusal("LICENSE-APACHE already exists and differs (use --force to overwrite)")
        );
        assert_eq!(
            plan(&targets, &dir, true),
            Ok(vec![Step::Create, Step::Overwrite])
        );
    }

    #[test]
    fn several_targets_are_planned_in_order() {
        let targets = [
            target("COPYING.LESSER", Some(("COPYING.LESSER", TEXT))),
            target("COPYING", None),
        ];
        let dir = files(&["COPYING.LESSER"]);
        assert_eq!(
            plan(&targets, &dir, false),
            Ok(vec![Step::Unchanged, Step::Create])
        );
    }

    #[test]
    fn an_explicit_output_path_is_planned_without_a_directory_listing() {
        let absent = [target("docs/NOTICE", None)];
        assert_eq!(plan(&absent, &[], false), Ok(vec![Step::Create]));
        let different = [target("docs/NOTICE", Some(("NOTICE", "old")))];
        assert_eq!(
            plan(&different, &[], false),
            refusal("docs/NOTICE already exists and differs (use --force to overwrite)")
        );
    }
}
