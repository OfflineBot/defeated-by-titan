//! The guard over the domain rule.
//!
//! **This rule rots quietly** — nothing breaks when somebody does reach across, and you only
//! notice once a domain can no longer be touched on its own. Hence this test: it reads the
//! files under `src/`, collects every `crate::<domain>` edge and falls over when one of them
//! is not on the allow list (`prompts/init.md` §5 rule 6).
//!
//! **The allow list lives in `docs/architecture.md`, not here.** A rule written down in two
//! places is wrong in one of them after four weeks — so the test reads the doc.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// What every domain may use without an entry of its own.
const FREE: [&str; 2] = ["shared", "data"];

fn domains() -> BTreeSet<String> {
    let mut d = BTreeSet::new();
    for entry in std::fs::read_dir(crate_root().join("src")).expect("src/ must be readable") {
        let entry = entry.expect("directory entry");
        if entry.path().is_dir() {
            d.insert(entry.file_name().to_string_lossy().to_string());
        }
    }
    d
}

fn rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut done = Vec::new();
    let mut pending = vec![dir.to_path_buf()];
    while let Some(p) = pending.pop() {
        for entry in std::fs::read_dir(&p).expect("directory readable") {
            let path = entry.expect("entry").path();
            if path.is_dir() {
                pending.push(path);
            } else if path.extension().is_some_and(|e| e == "rs") {
                done.push(path);
            }
        }
    }
    done
}

/// Reads the ```allowed block out of `docs/architecture.md`.
fn allow_list() -> BTreeSet<(String, String)> {
    let doc = std::fs::read_to_string(crate_root().join("docs/architecture.md"))
        .expect("docs/architecture.md must exist — the allow list lives there");
    let mut inside = false;
    let mut edges = BTreeSet::new();
    for line in doc.lines() {
        if line.trim_start().starts_with("```allowed") {
            inside = true;
            continue;
        }
        if inside && line.trim_start().starts_with("```") {
            break;
        }
        if !inside {
            continue;
        }
        let without_comment = line.split('#').next().unwrap_or("").trim();
        if without_comment.is_empty() {
            continue;
        }
        let Some((from, to)) = without_comment.split_once("->") else {
            panic!(
                "docs/architecture.md, allow list: {without_comment:?} does not match \
                 `from -> to   # reason`"
            );
        };
        edges.insert((from.trim().to_string(), to.trim().to_string()));
    }
    assert!(inside, "docs/architecture.md no longer has an ```allowed block");
    edges
}

#[test]
fn t003_no_domain_reaches_across_without_permission() {
    let domains = domains();
    let allowed = allow_list();
    let mut violations = Vec::new();

    for domain in &domains {
        if domain == "shared" {
            continue; // shared belongs to nobody and may know nobody
        }
        for file in rust_files(&crate_root().join("src").join(domain)) {
            let text = std::fs::read_to_string(&file).expect("file readable");
            for (no, line) in text.lines().enumerate() {
                // Doc comments may POINT AT other domains — a link is not a dependency.
                // Only real code counts.
                let code = line.trim_start();
                if code.starts_with("//") {
                    continue;
                }
                for target in &domains {
                    if target == domain || FREE.contains(&target.as_str()) {
                        continue;
                    }
                    let pattern = format!("crate::{target}::");
                    if code.contains(&pattern)
                        && !allowed.contains(&(domain.clone(), target.clone()))
                    {
                        violations.push(format!(
                            "{}:{} — {domain} reaches into {target}. Either go through a \
                             message in shared/, or add a line `{domain} -> {target}` \
                             with a reason to the allow list in docs/architecture.md",
                            file.strip_prefix(crate_root()).unwrap_or(&file).display(),
                            no + 1,
                        ));
                    }
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "{} edge(s) without permission:\n  {}",
        violations.len(),
        violations.join("\n  ")
    );
}

#[test]
fn t003_every_domain_has_exactly_one_plugin() {
    // A folder without a plugin is `shared/` or a mistake (§5 rule 1).
    let mut missing = Vec::new();
    for domain in domains() {
        if domain == "shared" {
            continue;
        }
        let text: String = rust_files(&crate_root().join("src").join(&domain))
            .iter()
            .map(|p| std::fs::read_to_string(p).expect("readable"))
            .collect();
        let has_plugin = text.contains("impl Plugin for");
        if !has_plugin {
            missing.push(domain);
        }
    }
    assert!(
        missing.is_empty(),
        "these folders under src/ have no `impl Plugin`: {missing:?} — \
         either they are shared/, or it is a mistake (init.md §5 rule 1)"
    );
}

#[test]
fn t003_the_allow_list_names_only_real_domains() {
    // A permission for a folder that does not (any longer) exist is a lie nobody notices —
    // until somebody reuses the name.
    let domains = domains();
    for (from, to) in allow_list() {
        assert!(domains.contains(&from), "the allow list names `{from}`, which does not exist");
        assert!(domains.contains(&to), "the allow list names `{to}`, which does not exist");
    }
}
