//! Static table of the supported licenses and their embedded texts.

/// A license text embedded in the binary. Licenses that share a text
/// (`-only` and `-or-later`) point to the same `Text`; `file` identifies it.
pub struct Text {
    pub file: &'static str,
    pub body: &'static str,
}

pub struct License {
    /// Canonical SPDX identifier.
    pub id: &'static str,
    pub name: &'static str,
    pub text: &'static Text,
}

macro_rules! texts {
    ($($name:ident = $file:literal;)*) => {$(
        static $name: Text = Text {
            file: $file,
            body: include_str!(concat!("../licenses/", $file)),
        };
    )*};
}

texts! {
    AGPL_3_0 = "agpl-3.0.txt";
    APACHE_2_0 = "apache-2.0.txt";
    BSD_0 = "0bsd.txt";
    BSD_2_CLAUSE = "bsd-2-clause.txt";
    BSD_3_CLAUSE = "bsd-3-clause.txt";
    BSL_1_0 = "bsl-1.0.txt";
    CC0_1_0 = "cc0-1.0.txt";
    CC_BY_4_0 = "cc-by-4.0.txt";
    CC_BY_NC_4_0 = "cc-by-nc-4.0.txt";
    CC_BY_NC_ND_4_0 = "cc-by-nc-nd-4.0.txt";
    CC_BY_NC_SA_4_0 = "cc-by-nc-sa-4.0.txt";
    CC_BY_ND_4_0 = "cc-by-nd-4.0.txt";
    CC_BY_SA_4_0 = "cc-by-sa-4.0.txt";
    EPL_2_0 = "epl-2.0.txt";
    EUPL_1_2 = "eupl-1.2.txt";
    GPL_2_0 = "gpl-2.0.txt";
    GPL_3_0 = "gpl-3.0.txt";
    ISC = "isc.txt";
    LGPL_2_1 = "lgpl-2.1.txt";
    LGPL_3_0 = "lgpl-3.0.txt";
    MIT = "mit.txt";
    MIT_0 = "mit-0.txt";
    MPL_2_0 = "mpl-2.0.txt";
    UNLICENSE = "unlicense.txt";
    ZLIB = "zlib.txt";
}

const fn license(id: &'static str, name: &'static str, text: &'static Text) -> License {
    License { id, name, text }
}

/// Sorted by identifier, ignoring case: `--list` prints the table as is.
#[rustfmt::skip]
pub static LICENSES: [License; 30] = [
    license("0BSD", "BSD Zero Clause License", &BSD_0),
    license("AGPL-3.0-only", "GNU Affero General Public License v3.0 only", &AGPL_3_0),
    license("AGPL-3.0-or-later", "GNU Affero General Public License v3.0 or later", &AGPL_3_0),
    license("Apache-2.0", "Apache License 2.0", &APACHE_2_0),
    license("BSD-2-Clause", "BSD 2-Clause \"Simplified\" License", &BSD_2_CLAUSE),
    license("BSD-3-Clause", "BSD 3-Clause \"New\" or \"Revised\" License", &BSD_3_CLAUSE),
    license("BSL-1.0", "Boost Software License 1.0", &BSL_1_0),
    license("CC-BY-4.0", "Creative Commons Attribution 4.0 International", &CC_BY_4_0),
    license("CC-BY-NC-4.0", "Creative Commons Attribution Non Commercial 4.0 International", &CC_BY_NC_4_0),
    license("CC-BY-NC-ND-4.0", "Creative Commons Attribution Non Commercial No Derivatives 4.0 International", &CC_BY_NC_ND_4_0),
    license("CC-BY-NC-SA-4.0", "Creative Commons Attribution Non Commercial Share Alike 4.0 International", &CC_BY_NC_SA_4_0),
    license("CC-BY-ND-4.0", "Creative Commons Attribution No Derivatives 4.0 International", &CC_BY_ND_4_0),
    license("CC-BY-SA-4.0", "Creative Commons Attribution Share Alike 4.0 International", &CC_BY_SA_4_0),
    license("CC0-1.0", "Creative Commons Zero v1.0 Universal", &CC0_1_0),
    license("EPL-2.0", "Eclipse Public License 2.0", &EPL_2_0),
    license("EUPL-1.2", "European Union Public License 1.2", &EUPL_1_2),
    license("GPL-2.0-only", "GNU General Public License v2.0 only", &GPL_2_0),
    license("GPL-2.0-or-later", "GNU General Public License v2.0 or later", &GPL_2_0),
    license("GPL-3.0-only", "GNU General Public License v3.0 only", &GPL_3_0),
    license("GPL-3.0-or-later", "GNU General Public License v3.0 or later", &GPL_3_0),
    license("ISC", "ISC License", &ISC),
    license("LGPL-2.1-only", "GNU Lesser General Public License v2.1 only", &LGPL_2_1),
    license("LGPL-2.1-or-later", "GNU Lesser General Public License v2.1 or later", &LGPL_2_1),
    license("LGPL-3.0-only", "GNU Lesser General Public License v3.0 only", &LGPL_3_0),
    license("LGPL-3.0-or-later", "GNU Lesser General Public License v3.0 or later", &LGPL_3_0),
    license("MIT", "MIT License", &MIT),
    license("MIT-0", "MIT No Attribution", &MIT_0),
    license("MPL-2.0", "Mozilla Public License 2.0", &MPL_2_0),
    license("Unlicense", "The Unlicense", &UNLICENSE),
    license("Zlib", "zlib License", &ZLIB),
];

/// Deprecated SPDX identifiers still accepted on input. Each one has an
/// `-only` and an `-or-later` successor in `LICENSES`.
const DEPRECATED: [&str; 5] = ["AGPL-3.0", "GPL-2.0", "GPL-3.0", "LGPL-2.1", "LGPL-3.0"];

/// Looks up a canonical identifier, ignoring case.
pub fn find(id: &str) -> Option<&'static License> {
    LICENSES.iter().find(|l| l.id.eq_ignore_ascii_case(id))
}

/// Returns the canonical spelling of `id` if it is a deprecated identifier.
pub fn deprecated(id: &str) -> Option<&'static str> {
    DEPRECATED.into_iter().find(|d| d.eq_ignore_ascii_case(id))
}

impl License {
    /// Suffix of the `LICENSE-<suffix>` file name.
    pub fn suffix(&self) -> &'static str {
        if self.id == "Apache-2.0" {
            "APACHE"
        } else {
            self.id
        }
    }
}

impl Text {
    /// The text that must be shipped next to this one: LGPL-3.0 only adds
    /// permissions on top of GPL-3.0 and is incomplete without it.
    pub fn companion(&self) -> Option<&'static Text> {
        (self.file == LGPL_3_0.file).then_some(&GPL_3_0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The licenses with a copyright line to fill, and that line.
    const COPYRIGHT_LINES: [(&str, &str); 7] = [
        ("MIT", "Copyright (c) [year] [fullname]"),
        ("ISC", "Copyright (c) [year] [fullname]"),
        ("0BSD", "Copyright (c) [year] [fullname]"),
        ("MIT-0", "Copyright [year] [fullname]"),
        ("BSD-2-Clause", "Copyright (c) [year], [fullname]"),
        ("BSD-3-Clause", "Copyright (c) [year], [fullname]"),
        ("Zlib", "(C) [year] [fullname]"),
    ];

    fn texts() -> Vec<&'static Text> {
        let mut texts: Vec<&Text> = Vec::new();
        for license in &LICENSES {
            if !texts.iter().any(|t| t.file == license.text.file) {
                texts.push(license.text);
            }
        }
        texts
    }

    #[test]
    fn every_identifier_resolves_in_any_case() {
        for license in &LICENSES {
            for spelling in [
                license.id.to_string(),
                license.id.to_lowercase(),
                license.id.to_uppercase(),
            ] {
                assert_eq!(find(&spelling).map(|l| l.id), Some(license.id));
            }
        }
        assert!(find("MTI").is_none());
        assert!(find("").is_none());
    }

    #[test]
    fn there_are_30_identifiers_for_25_texts() {
        let mut ids: Vec<&str> = LICENSES.iter().map(|l| l.id).collect();
        ids.dedup();
        assert_eq!(ids.len(), 30);
        assert_eq!(texts().len(), 25);
    }

    #[test]
    fn the_table_is_sorted_ignoring_case() {
        let ids: Vec<String> = LICENSES.iter().map(|l| l.id.to_lowercase()).collect();
        assert!(ids.is_sorted(), "{ids:?}");
    }

    #[test]
    fn deprecated_identifiers_have_both_successors() {
        for alias in DEPRECATED {
            assert_eq!(deprecated(&alias.to_lowercase()), Some(alias));
            assert!(find(alias).is_none(), "{alias} must not be canonical");
            for suffix in ["-only", "-or-later"] {
                assert!(find(&format!("{alias}{suffix}")).is_some());
            }
        }
        assert_eq!(deprecated("MIT"), None);
        assert_eq!(deprecated("GPL-3.0-only"), None);
    }

    #[test]
    fn only_apache_has_a_special_suffix() {
        for license in &LICENSES {
            let expected = if license.id == "Apache-2.0" {
                "APACHE"
            } else {
                license.id
            };
            assert_eq!(license.suffix(), expected);
        }
    }

    #[test]
    fn only_lgpl_3_has_a_companion_and_it_is_gpl_3() {
        for text in texts() {
            let expected = (text.file == "lgpl-3.0.txt").then_some("gpl-3.0.txt");
            assert_eq!(text.companion().map(|t| t.file), expected, "{}", text.file);
        }
    }

    #[test]
    fn placeholder_texts_have_exactly_the_expected_copyright_line() {
        for (id, line) in COPYRIGHT_LINES {
            let body = find(id).unwrap().text.body;
            assert_eq!(body.matches("[year]").count(), 1, "{id}");
            assert_eq!(body.matches("[fullname]").count(), 1, "{id}");
            assert_eq!(body.lines().filter(|l| *l == line).count(), 1, "{id}");
        }
    }

    #[test]
    fn other_texts_have_no_placeholder() {
        let others: Vec<&Text> = texts()
            .into_iter()
            .filter(|t| {
                COPYRIGHT_LINES
                    .iter()
                    .all(|(id, _)| find(id).unwrap().text.file != t.file)
            })
            .collect();
        assert_eq!(others.len(), 18);
        for text in others {
            assert!(!text.body.contains("[year]"), "{}", text.file);
            assert!(!text.body.contains("[fullname]"), "{}", text.file);
        }
    }

    #[test]
    fn apache_appendix_keeps_its_own_placeholders() {
        let body = find("Apache-2.0").unwrap().text.body;
        assert!(body.contains("Copyright [yyyy] [name of copyright owner]"));
    }

    #[test]
    fn texts_use_lf_and_end_with_exactly_one_newline() {
        for text in texts() {
            assert!(!text.body.contains('\r'), "{}", text.file);
            assert!(text.body.ends_with('\n'), "{}", text.file);
            assert!(!text.body.ends_with("\n\n"), "{}", text.file);
        }
    }
}
