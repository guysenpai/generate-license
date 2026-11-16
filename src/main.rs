//! Command line: arguments, year, holder and disk writes.

use std::fs::{self, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

use generate_license::registry::{LICENSES, License, Text};
use generate_license::{
    Error, Existing, Extraction, Step, Target, extract, name_files, needs_copyright, plan, render,
    year_of,
};
use lexopt::prelude::*;

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

enum Command {
    Help,
    Version,
    List,
    Generate(Args),
}

struct Args {
    expression: Vec<String>,
    year: Year,
    holder: Option<String>,
    output: Option<PathBuf>,
    force: bool,
}

enum Year {
    Current,
    Given(String),
    Omitted,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(Error::Failure(message)) => {
            eprintln!("error: {message}");
            ExitCode::from(1)
        }
        Err(Error::Usage(message)) => {
            eprintln!("error: {message}\nFor more information, try '--help'.");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), Error> {
    let command = parse(lexopt::Parser::from_env()).map_err(|e| Error::Usage(e.to_string()))?;
    match command {
        Command::Help => print!("{HELP}"),
        Command::Version => println!("generate-license {}", env!("CARGO_PKG_VERSION")),
        Command::List => list(),
        Command::Generate(args) => return generate(args),
    }
    Ok(())
}

fn generate(args: Args) -> Result<(), Error> {
    let Extraction { licenses, warnings } = extract(&args.expression.join(" "))?;
    for warning in warnings {
        eprintln!("warning: {warning}");
    }
    let files = name_files(&licenses, args.output.as_deref())?;
    let (year, holder) = copyright(&args, &licenses, &files)?;
    let rendered = files
        .into_iter()
        .map(|(path, text)| (path, render(text.body, year.as_deref(), &holder)));

    if args.output.as_deref() == Some(Path::new("-")) {
        let text: String = rendered.map(|(_, content)| content).collect();
        let printed = io::stdout().write_all(text.as_bytes());
        return printed.map_err(|e| Error::Failure(format!("cannot write to stdout: {e}")));
    }

    let targets = rendered
        .map(|(path, content)| {
            let existing = existing(&path)?;
            Ok(Target {
                path,
                content,
                existing,
            })
        })
        .collect::<Result<Vec<Target>, Error>>()?;
    let dir_files = match args.output {
        Some(_) => Vec::new(),
        None => regular_files().map_err(|e| failure("list the current directory", e))?,
    };
    let steps = plan(&targets, &dir_files, args.force)?;
    for (step, target) in steps.iter().zip(&targets) {
        apply(step, target, args.force)?;
    }
    Ok(())
}

fn parse(mut parser: lexopt::Parser) -> Result<Command, lexopt::Error> {
    let (mut expression, mut year, mut no_year) = (Vec::new(), None, false);
    let (mut holder, mut output, mut force, mut list) = (None, None, false, false);
    while let Some(arg) = parser.next()? {
        match arg {
            Long("year") => year = Some(parser.value()?.string()?),
            Long("no-year") => no_year = true,
            Long("holder") => holder = Some(parser.value()?.string()?),
            Short('o') | Long("output") => output = Some(parser.value()?.into()),
            Short('f') | Long("force") => force = true,
            Long("list") => list = true,
            Short('h') | Long("help") => return Ok(Command::Help),
            Short('V') | Long("version") => return Ok(Command::Version),
            Value(value) => expression.push(value.string()?),
            _ => return Err(arg.unexpected()),
        }
    }
    let year = match (year, no_year) {
        (Some(_), true) => return Err("--year and --no-year cannot be used together".into()),
        (Some(year), false) if year.is_empty() => {
            return Err("--year requires a non-empty value".into());
        }
        (Some(year), false) => Year::Given(year),
        (None, true) => Year::Omitted,
        (None, false) => Year::Current,
    };
    if holder.as_deref() == Some("") {
        return Err("--holder requires a non-empty value".into());
    }
    if !list {
        return Ok(Command::Generate(Args {
            expression,
            year,
            holder,
            output,
            force,
        }));
    }
    if !expression.is_empty() {
        return Err("--list does not take a license expression".into());
    }
    Ok(Command::List)
}

fn list() {
    let width = LICENSES.iter().map(|l| l.id.len()).max().unwrap_or(0);
    for license in &LICENSES {
        println!("{:<width$}  {}", license.id, license.name);
    }
}

/// Resolves the year and the holder, but only if a text has a copyright
/// line: `generate-license Apache-2.0` must work without git.
fn copyright(
    args: &Args,
    licenses: &[&License],
    files: &[(PathBuf, &Text)],
) -> Result<(Option<String>, String), Error> {
    if files.iter().any(|(_, text)| needs_copyright(text.body)) {
        let year = match &args.year {
            Year::Current => Some(current_year()),
            Year::Given(year) => Some(year.clone()),
            Year::Omitted => None,
        };
        return Ok((year, holder(args.holder.as_deref())?));
    }

    let year_flag = match args.year {
        Year::Current => None,
        Year::Given(_) => Some("--year"),
        Year::Omitted => Some("--no-year"),
    };
    let ids: Vec<&str> = licenses.iter().map(|l| l.id).collect();
    let verb = if ids.len() == 1 { "has" } else { "have" };
    for flag in [year_flag, args.holder.as_ref().map(|_| "--holder")]
        .into_iter()
        .flatten()
    {
        let ids = ids.join(", ");
        eprintln!("note: {flag} has no effect: {ids} {verb} no copyright line to fill");
    }
    Ok((None, String::new()))
}

fn current_year() -> String {
    let now = SystemTime::now().duration_since(UNIX_EPOCH);
    year_of(now.map_or(0, |elapsed| elapsed.as_secs())).to_string()
}

fn holder(given: Option<&str>) -> Result<String, Error> {
    if let Some(holder) = given {
        return Ok(holder.to_string());
    }
    let git = std::process::Command::new("git")
        .args(["config", "--get", "user.name"])
        .output();
    let name = git.map_or(String::new(), |output| {
        String::from_utf8_lossy(&output.stdout).trim().to_string()
    });
    if name.is_empty() {
        let message = "no copyright holder found: pass --holder or set git config user.name";
        return Err(Error::Failure(message.into()));
    }
    eprintln!("holder: {name} (from git config user.name)");
    Ok(name)
}

fn failure(action: &str, error: io::Error) -> Error {
    Error::Failure(format!("cannot {action}: {error}"))
}

fn existing(path: &Path) -> Result<Option<Existing>, Error> {
    let read = |error| failure(&format!("read {}", path.display()), error);
    let content = match fs::read(path) {
        Ok(content) => content,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(read(error)),
    };
    // On a case-insensitive file system `LICENSE` may be `license` on disk.
    let on_disk = fs::canonicalize(path).map_err(read)?;
    let name = on_disk.file_name().unwrap_or_default().to_string_lossy();
    Ok(Some(Existing {
        name: name.into_owned(),
        content,
    }))
}

fn regular_files() -> io::Result<Vec<String>> {
    let mut names = Vec::new();
    for entry in fs::read_dir(".")? {
        let entry = entry?;
        if entry.path().is_file() {
            names.push(entry.file_name().to_string_lossy().into_owned());
        }
    }
    Ok(names)
}

fn apply(step: &Step, target: &Target, force: bool) -> Result<(), Error> {
    let path = &target.path;
    let done = match step {
        Step::Unchanged => "unchanged",
        Step::Create | Step::Overwrite => {
            let mut options = OpenOptions::new();
            if force {
                options.create(true).truncate(true);
            } else {
                options.create_new(true);
            }
            let written = options
                .write(true)
                .open(path)
                .and_then(|mut file| file.write_all(target.content.as_bytes()));
            written.map_err(|e| failure(&format!("write {}", path.display()), e))?;
            if *step == Step::Create {
                "created"
            } else {
                "overwritten"
            }
        }
    };
    eprintln!("{done}: {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::HELP;

    /// The option names matched in this file, found by their pattern prefix.
    fn parsed_options(prefix: &str, quote: char) -> Vec<&'static str> {
        let source = include_str!("main.rs");
        let names = source.split(prefix).skip(1);
        names
            .map(|rest| rest.split(quote).next().unwrap())
            .collect()
    }

    #[test]
    fn help_documents_every_option_the_parser_accepts() {
        // Built at run time so that this test does not match itself.
        let short = parsed_options(&format!("Short{}", "('"), '\'');
        let long = parsed_options(&format!("Long{}", "(\""), '"');
        // Guard: if the parser moves or changes shape and nothing is found
        // any more, fail instead of checking an empty list.
        assert_eq!(long.len(), 8, "expected the parser's 8 options: {long:?}");
        assert_eq!(short, ["o", "f", "h", "V"]);
        for name in short {
            assert!(HELP.contains(&format!("\n  -{name}, --")), "-{name}");
        }
        for name in long {
            let documented = [' ', '\n']
                .iter()
                .any(|end| HELP.contains(&format!("--{name}{end}")));
            assert!(documented, "--{name}");
        }
    }
}
