//! Runs the real binary in a temporary directory with git isolated from
//! the machine, so no test depends on the developer's `user.name`.

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

const HELP: &str = r#"Generate license files from SPDX identifiers

Usage: generate-license [OPTIONS] <EXPRESSION>...
       generate-license --list

Arguments:
  <EXPRESSION>...  SPDX identifiers or expression, e.g. MIT OR Apache-2.0

Options:
      --year <YEAR>      Copyright year or range (default: current year)
      --no-year          Omit the year from the copyright line
      --holder <HOLDER>  Copyright holder (default: git config user.name)
  -o, --output <PATH>    Output path, "-" for stdout (single license only)
  -f, --force            Overwrite existing files
      --list             List supported licenses
  -h, --help             Print help
  -V, --version          Print version
"#;

const USAGE_HINT: &str = "For more information, try '--help'.\n";
const ADA: [&str; 4] = ["--holder", "Ada Lovelace", "--year", "2026"];

/// What one run of the binary produced.
struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

/// A temporary working directory, removed when the test ends.
struct Sandbox {
    root: PathBuf,
}

impl Sandbox {
    fn new() -> Self {
        static NEXT: AtomicUsize = AtomicUsize::new(0);
        let unique = format!(
            "generate-license-test-{}-{}",
            std::process::id(),
            NEXT.fetch_add(1, Ordering::Relaxed)
        );
        let root = std::env::temp_dir().join(unique);
        fs::create_dir_all(root.join("work")).unwrap();
        fs::write(root.join("gitconfig"), "").unwrap();
        Self { root }
    }

    fn path(&self, name: &str) -> PathBuf {
        self.root.join("work").join(name)
    }

    fn set_git_user(&self, name: &str) {
        let config = format!("[user]\n\tname = {name}\n");
        fs::write(self.root.join("gitconfig"), config).unwrap();
    }

    fn run(&self, args: &[&str]) -> Run {
        let output = Command::new(env!("CARGO_BIN_EXE_generate-license"))
            .args(args)
            .current_dir(self.root.join("work"))
            .env("GIT_CONFIG_GLOBAL", self.root.join("gitconfig"))
            .env("GIT_CONFIG_NOSYSTEM", "1")
            .env("GIT_CEILING_DIRECTORIES", &self.root)
            .output()
            .unwrap();
        Run {
            code: output.status.code().unwrap(),
            stdout: String::from_utf8(output.stdout).unwrap(),
            stderr: String::from_utf8(output.stderr).unwrap(),
        }
    }

    /// Runs with a fixed holder and year, and expects success.
    fn generate(&self, args: &[&str]) -> Run {
        let run = self.run(&[args, &ADA].concat());
        assert_eq!(run.code, 0, "{}", run.stderr);
        run
    }

    fn write(&self, name: &str, content: &str) {
        fs::write(self.path(name), content).unwrap();
    }

    fn read(&self, name: &str) -> String {
        fs::read_to_string(self.path(name)).unwrap()
    }

    fn files(&self) -> Vec<String> {
        let entries = fs::read_dir(self.root.join("work")).unwrap();
        let mut names: Vec<String> = entries
            .map(|e| e.unwrap().file_name().into_string().unwrap())
            .collect();
        names.sort();
        names
    }
}

impl Drop for Sandbox {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.root);
    }
}

/// An upstream text, read from the repository rather than from the binary.
fn source(file: &str) -> String {
    let path = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("licenses")
        .join(file);
    fs::read_to_string(path).unwrap()
}

fn mit(copyright_line: &str) -> String {
    source("mit.txt").replace("Copyright (c) [year] [fullname]", copyright_line)
}

#[test]
fn writes_then_leaves_unchanged_then_refuses_then_overwrites() {
    let sandbox = Sandbox::new();
    let ada = mit("Copyright (c) 2026 Ada Lovelace");

    let first = sandbox.generate(&["MIT"]);
    assert_eq!(first.stderr, "created: LICENSE\n");
    assert_eq!(first.stdout, "");
    assert_eq!(sandbox.read("LICENSE"), ada);

    let second = sandbox.generate(&["MIT"]);
    assert_eq!(second.stderr, "unchanged: LICENSE\n");
    assert_eq!(sandbox.read("LICENSE"), ada);

    let grace = ["MIT", "--holder", "Grace Hopper", "--year", "2026"];
    let refused = sandbox.run(&grace);
    assert_eq!(refused.code, 1);
    assert_eq!(
        refused.stderr,
        "error: LICENSE already exists and differs (use --force to overwrite)\n"
    );
    assert_eq!(sandbox.read("LICENSE"), ada);

    let forced = sandbox.run(&[&grace[..], &["--force"]].concat());
    assert_eq!(forced.code, 0);
    assert_eq!(forced.stderr, "overwritten: LICENSE\n");
    assert_eq!(
        sandbox.read("LICENSE"),
        mit("Copyright (c) 2026 Grace Hopper")
    );
    assert_eq!(sandbox.files(), ["LICENSE"]);
}

#[test]
fn force_leaves_identical_files_unchanged_and_creates_missing_ones() {
    let sandbox = Sandbox::new();
    assert_eq!(
        sandbox.generate(&["MIT", "-f"]).stderr,
        "created: LICENSE\n"
    );
    assert_eq!(
        sandbox.generate(&["MIT", "-f"]).stderr,
        "unchanged: LICENSE\n"
    );
}

#[test]
fn several_licenses_get_one_file_each() {
    for expression in [&["MIT", "OR", "Apache-2.0"][..], &["(MIT OR Apache-2.0)"]] {
        let sandbox = Sandbox::new();
        let run = sandbox.generate(expression);
        assert_eq!(
            run.stderr,
            "created: LICENSE-MIT\ncreated: LICENSE-APACHE\n"
        );
        assert_eq!(
            sandbox.read("LICENSE-MIT"),
            mit("Copyright (c) 2026 Ada Lovelace")
        );
        assert_eq!(sandbox.read("LICENSE-APACHE"), source("apache-2.0.txt"));
        assert_eq!(sandbox.files(), ["LICENSE-APACHE", "LICENSE-MIT"]);
    }
}

#[test]
fn nothing_is_written_if_one_of_several_files_differs() {
    let sandbox = Sandbox::new();
    sandbox.write("LICENSE-APACHE", "old");
    let run = sandbox.run(&[&["MIT", "OR", "Apache-2.0"], &ADA[..]].concat());
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stderr,
        "error: LICENSE-APACHE already exists and differs (use --force to overwrite)\n"
    );
    assert_eq!(sandbox.files(), ["LICENSE-APACHE"]);
    assert_eq!(sandbox.read("LICENSE-APACHE"), "old");
}

#[test]
fn another_license_file_is_refused_unless_output_or_force_is_given() {
    let sandbox = Sandbox::new();
    sandbox.write("LICENSE.md", "old");
    sandbox.write("README.md", "readme");
    fs::create_dir(sandbox.path("COPYING")).unwrap();

    let refused = sandbox.run(&[&["MIT"], &ADA[..]].concat());
    assert_eq!(refused.code, 1);
    assert_eq!(
        refused.stderr,
        "error: other license files found: LICENSE.md (remove them, use -o, or pass --force)\n"
    );
    assert_eq!(sandbox.files(), ["COPYING", "LICENSE.md", "README.md"]);

    let explicit = sandbox.generate(&["MIT", "-o", "NOTICE.txt"]);
    assert_eq!(explicit.stderr, "created: NOTICE.txt\n");
    assert_eq!(
        sandbox.read("NOTICE.txt"),
        mit("Copyright (c) 2026 Ada Lovelace")
    );

    let forced = sandbox.generate(&["MIT", "--force"]);
    assert_eq!(forced.stderr, "created: LICENSE\n");
    assert_eq!(sandbox.read("LICENSE.md"), "old");
}

#[test]
fn an_explicit_output_follows_the_same_existing_file_rules() {
    let sandbox = Sandbox::new();
    fs::create_dir(sandbox.path("docs")).unwrap();
    let args = ["MIT", "--output", "docs/NOTICE"];
    assert_eq!(sandbox.generate(&args).stderr, "created: docs/NOTICE\n");
    assert_eq!(sandbox.generate(&args).stderr, "unchanged: docs/NOTICE\n");

    let refused = sandbox.run(&[&args[..], &["--holder", "Grace Hopper"]].concat());
    assert_eq!(refused.code, 1);
    assert_eq!(
        refused.stderr,
        "error: docs/NOTICE already exists and differs (use --force to overwrite)\n"
    );
}

#[test]
fn the_parent_directory_of_an_output_is_not_created() {
    let sandbox = Sandbox::new();
    let run = sandbox.run(&[&["MIT", "-o", "missing/LICENSE"], &ADA[..]].concat());
    assert_eq!(run.code, 1);
    assert!(
        run.stderr
            .starts_with("error: cannot write missing/LICENSE: "),
        "{}",
        run.stderr
    );
    assert!(sandbox.files().is_empty());
}

#[test]
fn output_dash_prints_the_text_and_writes_nothing() {
    let sandbox = Sandbox::new();
    sandbox.write("LICENSE", "old");

    let run = sandbox.generate(&["MIT", "-o", "-"]);
    assert_eq!(run.stdout, mit("Copyright (c) 2026 Ada Lovelace"));
    assert_eq!(run.stderr, "");

    let apache = sandbox.run(&["Apache-2.0", "-o", "-"]);
    assert_eq!(apache.stdout, source("apache-2.0.txt"));

    assert_eq!(sandbox.files(), ["LICENSE"]);
    assert_eq!(sandbox.read("LICENSE"), "old");
}

#[test]
fn lgpl_3_is_written_with_the_gpl_3_text() {
    let sandbox = Sandbox::new();
    let run = sandbox.run(&["LGPL-3.0-only"]);
    assert_eq!(run.code, 0);
    assert_eq!(run.stderr, "created: COPYING.LESSER\ncreated: COPYING\n");
    assert_eq!(sandbox.read("COPYING.LESSER"), source("lgpl-3.0.txt"));
    assert_eq!(sandbox.read("COPYING"), source("gpl-3.0.txt"));

    let again = sandbox.run(&["GPL-3.0-only OR LGPL-3.0-or-later"]);
    assert_eq!(
        again.stderr,
        "unchanged: COPYING.LESSER\nunchanged: COPYING\n"
    );
}

#[test]
fn a_deprecated_identifier_warns_and_is_written() {
    let sandbox = Sandbox::new();
    let run = sandbox.run(&["gpl-3.0+"]);
    assert_eq!(run.code, 0);
    assert_eq!(
        run.stderr,
        "warning: 'GPL-3.0+' is a deprecated SPDX identifier, using 'GPL-3.0-or-later'\n\
         created: LICENSE\n"
    );
    assert_eq!(sandbox.read("LICENSE"), source("gpl-3.0.txt"));
}

#[test]
fn the_year_can_be_a_range_or_left_out() {
    let sandbox = Sandbox::new();
    let range = [
        "BSD-3-Clause",
        "--holder",
        "Ada Lovelace",
        "-o",
        "-",
        "--year",
        "2019-2026",
    ];
    let line = |run: Run| run.stdout.lines().nth(2).unwrap().to_string();
    assert_eq!(
        line(sandbox.run(&range)),
        "Copyright (c) 2019-2026, Ada Lovelace"
    );

    let none = [
        "BSD-3-Clause",
        "--holder",
        "Ada Lovelace",
        "-o",
        "-",
        "--no-year",
    ];
    assert_eq!(line(sandbox.run(&none)), "Copyright (c) Ada Lovelace");
}

#[test]
fn the_year_defaults_to_the_current_one() {
    let sandbox = Sandbox::new();
    let run = sandbox.run(&["MIT", "--holder", "Ada Lovelace", "-o", "-"]);
    let line = run.stdout.lines().nth(2).unwrap();
    let year = line
        .strip_prefix("Copyright (c) ")
        .and_then(|rest| rest.strip_suffix(" Ada Lovelace"))
        .unwrap_or_else(|| panic!("unexpected copyright line: {line}"));
    assert_eq!(year.len(), 4, "{line}");
    assert!(year.chars().all(|c| c.is_ascii_digit()), "{line}");
    assert!(year >= "2026", "{line}");
}

#[test]
fn the_holder_defaults_to_the_git_user_name() {
    let sandbox = Sandbox::new();
    sandbox.set_git_user("Ada Lovelace");
    for args in [
        &["MIT", "--year", "2026"][..],
        &["MIT", "--year", "2026", "--holder", ""],
    ] {
        let run = sandbox.run(args);
        assert_eq!(run.code, 0);
        assert_eq!(
            run.stderr.lines().next(),
            Some("holder: Ada Lovelace (from git config user.name)")
        );
        assert_eq!(run.stdout, "");
        assert_eq!(
            sandbox.read("LICENSE"),
            mit("Copyright (c) 2026 Ada Lovelace")
        );
    }
}

#[test]
fn a_missing_holder_is_an_error_only_when_a_text_needs_one() {
    let sandbox = Sandbox::new();
    let run = sandbox.run(&["MIT"]);
    assert_eq!(run.code, 1);
    assert_eq!(
        run.stderr,
        "error: no copyright holder found: pass --holder or set git config user.name\n"
    );
    assert!(sandbox.files().is_empty());

    let apache = sandbox.run(&["Apache-2.0"]);
    assert_eq!(apache.code, 0);
    assert_eq!(apache.stderr, "created: LICENSE\n");
    assert_eq!(sandbox.read("LICENSE"), source("apache-2.0.txt"));
}

#[test]
fn copyright_options_without_a_copyright_line_only_get_a_note() {
    let sandbox = Sandbox::new();
    let one = sandbox.run(&["Apache-2.0", "--holder", "Ada Lovelace", "-o", "-"]);
    assert_eq!(one.code, 0);
    assert_eq!(
        one.stderr,
        "note: --holder has no effect: Apache-2.0 has no copyright line to fill\n"
    );

    let two = sandbox.run(&["MPL-2.0 OR Apache-2.0", "--no-year"]);
    assert_eq!(two.code, 0);
    assert_eq!(
        two.stderr.lines().next(),
        Some("note: --no-year has no effect: MPL-2.0, Apache-2.0 have no copyright line to fill")
    );

    let both = sandbox.run(&["CC0-1.0", "--year", "2026", "--holder", "Ada", "-o", "-"]);
    assert_eq!(
        both.stderr,
        "note: --year has no effect: CC0-1.0 has no copyright line to fill\n\
         note: --holder has no effect: CC0-1.0 has no copyright line to fill\n"
    );

    let mixed = sandbox.generate(&["MIT OR Apache-2.0", "--force"]);
    assert!(!mixed.stderr.contains("note:"), "{}", mixed.stderr);
}

#[test]
fn a_file_differing_only_by_case_is_the_target_where_the_file_system_says_so() {
    let sandbox = Sandbox::new();
    sandbox.write("license", &mit("Copyright (c) 2026 Ada Lovelace"));
    let case_insensitive = sandbox.path("LICENSE").exists();

    let run = sandbox.run(&[&["MIT"], &ADA[..]].concat());
    if case_insensitive {
        assert_eq!(run.stderr, "unchanged: LICENSE\n");
        assert_eq!(run.code, 0);
    } else {
        assert_eq!(
            run.stderr,
            "error: other license files found: license (remove them, use -o, or pass --force)\n"
        );
        assert_eq!(run.code, 1);
    }
    assert_eq!(sandbox.files(), ["license"]);
}

#[test]
fn list_prints_the_30_identifiers_sorted_and_aligned() {
    let sandbox = Sandbox::new();
    let run = sandbox.run(&["--list"]);
    assert_eq!(run.code, 0);
    assert_eq!(run.stderr, "");

    let lines: Vec<&str> = run.stdout.lines().collect();
    assert_eq!(lines.len(), 30);
    assert_eq!(lines[0], "0BSD               BSD Zero Clause License");
    assert_eq!(
        lines[2],
        "AGPL-3.0-or-later  GNU Affero General Public License v3.0 or later"
    );
    assert_eq!(lines[29], "Zlib               zlib License");

    let ids: Vec<String> = lines
        .iter()
        .map(|line| line.split(' ').next().unwrap().to_lowercase())
        .collect();
    assert!(ids.is_sorted(), "{ids:?}");
    for line in lines {
        assert_eq!(&line[17..19], "  ", "{line}");
        assert_ne!(&line[19..20], " ", "{line}");
    }
}

#[test]
fn help_prints_the_documented_text() {
    let sandbox = Sandbox::new();
    for flag in ["--help", "-h"] {
        let run = sandbox.run(&["MIT", flag]);
        assert_eq!(run.code, 0);
        assert_eq!(run.stdout, HELP);
        assert_eq!(run.stderr, "");
    }
    assert!(sandbox.files().is_empty());
}

#[test]
fn version_prints_the_package_version() {
    let sandbox = Sandbox::new();
    for flag in ["--version", "-V"] {
        let run = sandbox.run(&[flag]);
        assert_eq!(run.code, 0);
        assert_eq!(
            run.stdout,
            concat!("generate-license ", env!("CARGO_PKG_VERSION"), "\n")
        );
        assert_eq!(run.stderr, "");
    }
}

#[test]
fn usage_errors_exit_with_2_and_point_to_help() {
    let sandbox = Sandbox::new();
    for (args, message) in [
        (&[][..], "missing license identifier (try --list)"),
        (&["OR"], "missing license identifier (try --list)"),
        (
            &["MIT", "--year", "2026", "--no-year"],
            "--year and --no-year cannot be used together",
        ),
        (&["MIT", "--year", ""], "--year must not be empty"),
        (
            &["--list", "MIT"],
            "--list does not take a license expression",
        ),
        (&["MIT", "--bogus"], "invalid option '--bogus'"),
        (&["MIT", "-x"], "invalid option '-x'"),
        (&["MIT", "--year"], "missing argument for option '--year'"),
        (&["MIT", "-o"], "missing argument for option '-o'"),
        (
            &["MIT", "--force=yes"],
            r#"unexpected argument for option '--force': "yes""#,
        ),
        (
            &["MIT OR Apache-2.0", "-o", "out.txt"],
            "--output takes a single file, but 2 would be written: LICENSE-MIT, LICENSE-APACHE",
        ),
        (
            &["LGPL-3.0-only", "-o", "-"],
            "--output takes a single file, but 2 would be written: COPYING.LESSER, COPYING",
        ),
    ] {
        let run = sandbox.run(args);
        assert_eq!(run.code, 2, "{args:?}");
        assert_eq!(
            run.stderr,
            format!("error: {message}\n{USAGE_HINT}"),
            "{args:?}"
        );
        assert_eq!(run.stdout, "", "{args:?}");
    }
    assert!(sandbox.files().is_empty());
}

#[test]
fn unknown_licenses_and_exceptions_exit_with_1() {
    let sandbox = Sandbox::new();
    let unknown = sandbox.run(&[&["MTI"], &ADA[..]].concat());
    assert_eq!(unknown.code, 1);
    assert_eq!(
        unknown.stderr,
        "error: unknown or unsupported license 'MTI'\n\
         tip: did you mean 'MIT'?\n\
         Run 'generate-license --list' to see supported licenses.\n"
    );

    let exception = sandbox.run(&["GPL-2.0-only WITH Classpath-exception-2.0"]);
    assert_eq!(exception.code, 1);
    assert_eq!(
        exception.stderr,
        "error: license exceptions (WITH) are not supported\n"
    );
    assert!(sandbox.files().is_empty());
}
