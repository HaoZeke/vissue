//! A slice of the seat, packed so somebody else can open it.
//!
//! "Give the new person project X and tasks Y" is not one of the questions any
//! of these tools answers. The tracker knows the work, the deed store knows
//! what the work produced, and the pack knows what was learned; handing over a
//! piece of that means taking a slice across all three and making it stand on
//! its own somewhere else.
//!
//! What a slice has to carry, beyond the issues somebody named:
//!
//! - what they stand on. An issue whose blockers are absent is a task with no
//!   account of why it is not done, so the closure walks blockers.
//! - what plan they belong to. A child with no parent is a task with no reason.
//! - what the work produced, by accession. The deeds themselves come from the
//!   deed store, which is the only thing that can vouch for them; this names
//!   them and the receiver fetches or refuses.
//!
//! The shape is BagIt (RFC 8493): a `data/` payload, a manifest of every file
//! in it with a digest, and a `bag-info.txt` saying who packed it and when. A
//! receiver checks the manifest before reading anything, which is the property
//! a tarball does not have. `satchel.json` beside the payload is the
//! self-description RO-Crate argues a package needs
//! (doi:10.3233/DS-210053): the parts alone do not say what the whole was
//! meant to be, or which of the parts were asked for rather than pulled in.
//!
//! This packs what the tracker holds. Deed bytes are the deed store's to
//! export and atoms are the pack's, and `needs` is the list the deed store
//! takes, so the three halves compose on pipes without this crate depending on
//! either of them:
//!
//! ```console
//! $ vissue satchel --out bag --project x --issue y
//! $ packset export --into bag/data/atoms | deedar export --into bag/data/deeds -
//! $ jq -r '.needs[]' bag/data/satchel.json | deedar export --into bag/data/deeds -
//! $ vissue satchel --seal bag && vissue satchel --verify bag
//! ```
//!
//! The accession is what makes that work. It is the one identifier crossing
//! all three stores, so each of them can name what it needs from the others
//! without reading their formats.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::config::Layout;
use crate::error::{Error, Result};
use crate::views::IssueRec;

/// The format this writes, so a reader that meets a later one can say so
/// rather than guess.
pub const VERSION: &str = "vissue-satchel/1";

/// What was asked for, as opposed to what came along with it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Slice {
    /// Projects taken whole.
    pub projects: Vec<String>,
    /// Issues named one at a time.
    pub issues: Vec<String>,
}

impl Slice {
    /// Whether this names nothing, which is not a slice.
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.projects.is_empty() && self.issues.is_empty()
    }
}

/// The self-description written beside the payload.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Satchel {
    /// Format tag.
    pub version: String,
    /// What the packer was asked for.
    pub asked: Slice,
    /// Every issue in the closure, in id order.
    pub issues: Vec<String>,
    /// Issues that came along because something named needed them.
    pub carried: Vec<String>,
    /// Deed accessions the issues cite, which the deed store has to supply.
    pub needs: Vec<String>,
    /// Who packed it.
    pub packed_by: String,
    /// When, as an org inactive timestamp.
    pub packed_at: String,
}

/// What a pack or a check found.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Report {
    /// Issues written.
    pub issues: usize,
    /// Deed accessions named.
    pub needs: usize,
    /// Files in the payload.
    pub files: usize,
    /// Anything a receiver should be told, in the order it was found.
    pub notes: Vec<String>,
}

impl Report {
    /// One line per fact, which is what a command prints.
    #[must_use]
    pub fn render(&self) -> String {
        let mut out = format!(
            "issues={} deeds={} files={}\n",
            self.issues, self.needs, self.files
        );
        for note in &self.notes {
            out.push_str(note);
            out.push('\n');
        }
        out
    }
}

/// Pack a slice of the tracker into `dest`.
///
/// The closure is the issues named, plus every blocker they stand on and every
/// parent they sit under, walked to a fixed point. Children are not pulled in:
/// handing over a plan means handing over the plan, and a parent's other
/// children are other people's work.
///
/// # Errors
///
/// Returns an error when the slice names nothing, when an issue is not in the
/// corpus, or when `dest` cannot be written.
pub fn pack(layout: &Layout, asked: &Slice, dest: &Path) -> Result<Report> {
    if asked.is_empty() {
        return Err(Error::Other(anyhow::anyhow!(
            "a satchel needs a project or an issue to pack"
        )));
    }
    let recs = crate::catalog::load_recs(layout)?;
    let by_id: BTreeMap<&str, &IssueRec> =
        recs.iter().map(|r| (r.heading.id.as_str(), r)).collect();

    // What was named.
    let mut named: BTreeSet<String> = BTreeSet::new();
    for project in &asked.projects {
        for rec in &recs {
            if rec.project == *project {
                named.insert(rec.heading.id.clone());
            }
        }
    }
    for id in &asked.issues {
        if !by_id.contains_key(id.as_str()) {
            return Err(Error::IssueNotFound { id: id.clone() });
        }
        named.insert(id.clone());
    }
    if named.is_empty() {
        return Err(Error::Other(anyhow::anyhow!(
            "nothing to pack: {:?} matched no issue",
            asked.projects
        )));
    }

    // Everything they stand on, to a fixed point.
    let mut chosen = named.clone();
    let mut frontier: Vec<String> = named.iter().cloned().collect();
    while let Some(id) = frontier.pop() {
        let Some(rec) = by_id.get(id.as_str()) else {
            continue;
        };
        let mut reach: Vec<String> = rec.heading.blocked_by();
        if let Some(parent) = rec.heading.parent().map(str::to_string) {
            reach.push(parent);
        }
        for next in reach {
            if by_id.contains_key(next.as_str()) && chosen.insert(next.clone()) {
                frontier.push(next);
            }
        }
    }

    let carried: Vec<String> = chosen.difference(&named).cloned().collect();
    let mut needs: BTreeSet<String> = BTreeSet::new();
    for id in &chosen {
        if let Some(rec) = by_id.get(id.as_str()) {
            needs.extend(rec.heading.deeds());
        }
    }

    // Payload first, manifest over what was written.
    let data = dest.join("data");
    let issues_dir = data.join("issues");
    std::fs::create_dir_all(&issues_dir).map_err(Error::from)?;
    let mut payload: Vec<(PathBuf, String)> = Vec::new();
    for id in &chosen {
        let Some(rec) = by_id.get(id.as_str()) else {
            continue;
        };
        let text = crate::catalog::org_text_from(rec)?;
        let rel = PathBuf::from("data/issues").join(format!("{id}.org"));
        write_payload(dest, &rel, text.as_bytes())?;
        payload.push((rel, digest(text.as_bytes())));
    }

    let satchel = Satchel {
        version: VERSION.to_string(),
        asked: asked.clone(),
        issues: chosen.iter().cloned().collect(),
        carried,
        needs: needs.iter().cloned().collect(),
        packed_by: crate::config::identity(layout),
        packed_at: crate::model::today_inactive_bracket(),
    };
    let described = serde_json::to_string_pretty(&satchel)
        .map(|json| json + "\n")
        .map_err(|e| Error::Other(anyhow::anyhow!("{e}")))?;
    let rel = PathBuf::from("data/satchel.json");
    write_payload(dest, &rel, described.as_bytes())?;
    payload.push((rel, digest(described.as_bytes())));

    payload.sort();
    write_manifest(dest, &payload)?;
    write_declaration(dest, &satchel)?;

    let mut notes = Vec::new();
    if !satchel.needs.is_empty() {
        notes.push(format!(
            "{} deed accessions are named and not enclosed; the deed store exports them",
            satchel.needs.len()
        ));
    }
    if !satchel.carried.is_empty() {
        notes.push(format!(
            "{} issues came along as blockers or parents of what was asked for",
            satchel.carried.len()
        ));
    }
    Ok(Report {
        issues: satchel.issues.len(),
        needs: satchel.needs.len(),
        files: payload.len(),
        notes,
    })
}

/// Re-manifest a satchel over everything now in its payload.
///
/// Packing writes what the tracker holds, and the deed store fills in the
/// deeds afterwards, so the manifest written at pack time covers less than the
/// satchel ends up carrying. Sealing is the step that says the payload is
/// complete: after it, `verify` accounts for every file, and a file added
/// later shows up as unlisted.
///
/// # Errors
///
/// Returns an error when the directory is not a satchel or cannot be read.
pub fn seal(dir: &Path) -> Result<Report> {
    let satchel = describe(dir)?;
    let mut payload: Vec<(PathBuf, String)> = Vec::new();
    for found in walk(&dir.join("data"))? {
        let rel = found
            .strip_prefix(dir)
            .map_err(|e| Error::Other(anyhow::anyhow!("{e}")))?
            .to_path_buf();
        let bytes = std::fs::read(&found).map_err(Error::from)?;
        payload.push((rel, digest(&bytes)));
    }
    payload.sort();
    write_manifest(dir, &payload)?;

    let mut notes = shortfall(dir, &satchel);
    let atoms = atom_lines(dir);
    if atoms > 0 {
        notes.push(format!("{atoms} atoms arrived from a pack"));
    }
    Ok(Report {
        issues: satchel.issues.len(),
        needs: satchel.needs.len(),
        files: payload.len(),
        notes,
    })
}

/// What the description names and the payload does not hold.
///
/// A satchel that names a deed and does not carry it is not broken, because
/// the deed store may not have been asked yet. It is worth saying out loud, so
/// the receiver learns it from the check rather than from opening one.
fn shortfall(dir: &Path, satchel: &Satchel) -> Vec<String> {
    let enclosed = enclosed_deeds(dir);
    let mut out = Vec::new();
    let short = satchel
        .needs
        .iter()
        .filter(|acc| !enclosed.contains(*acc))
        .count();
    if short > 0 {
        out.push(format!(
            "{short} of {} deed accessions are named and not enclosed",
            satchel.needs.len()
        ));
    }
    out.extend(provenance_note(dir, &enclosed));
    out
}

/// What this check did and did not establish about who packed the satchel.
///
/// The payload matching the manifest says the bag arrived as it was written.
/// It says nothing about who wrote it, because a receiver recomputing digests
/// from the bag they were handed is checking the bag against itself. That
/// second question needs the signature over the manifest, and the keys a
/// reader accepts live in their deed store rather than here, so this names
/// what is left rather than answering it.
///
/// Saying so is the point. A check that reports "ok" for the first question
/// and stays quiet about the second invites the reader to hear both.
fn signature_note(dir: &Path) -> String {
    let manifest = dir.join("manifest-sha256.txt");
    let signature = manifest.with_extension("txt.sig");
    if signature.is_file() {
        format!(
            "the payload matches the manifest, and the manifest carries a signature this check \
             did not verify: `deedar vouch check {}` says whether a key you accept made it",
            manifest.display()
        )
    } else {
        "the payload matches the manifest, and the manifest is unsigned, so this establishes \
         that the bag arrived as written and nothing about who wrote it"
            .to_string()
    }
}

/// How many atoms the pack put in, counted rather than parsed: the receiver
/// wants to know something came, and reading them is their business.
fn atom_lines(dir: &Path) -> usize {
    let Ok(entries) = std::fs::read_dir(dir.join("data").join("atoms")) else {
        return 0;
    };
    entries
        .flatten()
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .map(|text| text.lines().filter(|l| !l.trim().is_empty()).count())
        .sum()
}

/// Which accessions actually have a directory under the payload.
/// What the enclosed deeds still need checking for, and by what.
///
/// A satchel check is a check of the bag against its own manifest. It catches
/// a payload that was corrupted or truncated and it cannot, even in principle,
/// say whether a deed inside predates somebody asking for it: a sender who
/// mints a deed the morning of the handover writes a manifest that agrees with
/// it perfectly.
///
/// The deed store answers that, with the inclusion proof each deed travels
/// with. Reading one is the deed store's business and not this crate's, so
/// what belongs here is the fact that the question is open and the name of the
/// verb that closes it. A receiver told only "the bag checks out" reads that
/// as more than it says.
fn provenance_note(dir: &Path, enclosed: &BTreeSet<String>) -> Option<String> {
    if enclosed.is_empty() {
        return None;
    }
    let deeds = dir.join("data").join("deeds");
    let bare: Vec<&String> = enclosed
        .iter()
        .filter(|acc| !deeds.join(acc).join("proof.txt").is_file())
        .collect();
    if bare.is_empty() {
        return Some(format!(
            "{} deeds arrived carrying a proof this check does not read: \
             `deedar check {}` says whether the sender's log held them before \
             the handover",
            enclosed.len(),
            dir.display()
        ));
    }
    Some(format!(
        "{} of {} enclosed deeds carry no proof, so nothing says they were \
         logged before they were handed over: {}",
        bare.len(),
        enclosed.len(),
        bare.iter()
            .map(|acc| acc.as_str())
            .collect::<Vec<_>>()
            .join(", ")
    ))
}

fn enclosed_deeds(dir: &Path) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    let Ok(entries) = std::fs::read_dir(dir.join("data").join("deeds")) else {
        return out;
    };
    for entry in entries.flatten() {
        if entry.path().is_dir()
            && let Some(name) = entry.file_name().to_str()
        {
            out.insert(name.to_string());
        }
    }
    out
}

/// Check a satchel: every file the manifest names is present and hashes right,
/// and nothing in the payload is unaccounted for.
///
/// The second half is the one that matters. A manifest that only proves what
/// it lists would let a packer add a file nobody agreed to, which is how a
/// bundle becomes a delivery mechanism.
///
/// # Errors
///
/// Returns an error when the satchel cannot be read or does not check out.
pub fn verify(dir: &Path) -> Result<Report> {
    let manifest = dir.join("manifest-sha256.txt");
    let text = std::fs::read_to_string(&manifest)
        .map_err(|_| Error::Other(anyhow::anyhow!("no manifest at {}", manifest.display())))?;
    let mut listed: BTreeMap<PathBuf, String> = BTreeMap::new();
    for line in text.lines().filter(|l| !l.trim().is_empty()) {
        let Some((hash, rel)) = line.split_once("  ") else {
            return Err(Error::Other(anyhow::anyhow!(
                "manifest: not an entry: {line:?}"
            )));
        };
        listed.insert(PathBuf::from(rel), hash.to_string());
    }

    let mut notes = Vec::new();
    for (rel, want) in &listed {
        let path = dir.join(rel);
        let Ok(bytes) = std::fs::read(&path) else {
            notes.push(format!("missing {}", rel.display()));
            continue;
        };
        let got = digest(&bytes);
        if got != *want {
            notes.push(format!("changed {}", rel.display()));
        }
    }
    for found in walk(&dir.join("data"))? {
        let rel = found
            .strip_prefix(dir)
            .map_err(|e| Error::Other(anyhow::anyhow!("{e}")))?
            .to_path_buf();
        if !listed.contains_key(&rel) {
            notes.push(format!("unlisted {}", rel.display()));
        }
    }

    let described = dir.join("data").join("satchel.json");
    let satchel: Satchel = std::fs::read_to_string(&described)
        .map_err(|_| Error::Other(anyhow::anyhow!("no data/satchel.json")))
        .and_then(|raw| serde_json::from_str(&raw).map_err(Error::from))?;
    if satchel.version != VERSION {
        notes.push(format!(
            "packed as {} and read as {VERSION}",
            satchel.version
        ));
    }

    if notes.is_empty() {
        // A named deed that never arrived is a note, not a failure: the deed
        // store may not have been asked. What arrived and does not check out
        // is the failure, and that is already above.
        let mut notes = shortfall(dir, &satchel);
        let atoms = atom_lines(dir);
        if atoms > 0 {
            notes.push(format!("{atoms} atoms arrived from a pack"));
        }
        notes.push(signature_note(dir));
        Ok(Report {
            issues: satchel.issues.len(),
            needs: satchel.needs.len(),
            files: listed.len(),
            notes,
        })
    } else {
        Err(Error::Other(anyhow::anyhow!(
            "the satchel does not check out:\n{}",
            notes.join("\n")
        )))
    }
}

/// Read a satchel's description without checking it.
///
/// # Errors
///
/// Returns an error when the file is absent or is not a satchel.
pub fn describe(dir: &Path) -> Result<Satchel> {
    let path = dir.join("data").join("satchel.json");
    let raw = std::fs::read_to_string(&path)
        .map_err(|_| Error::Other(anyhow::anyhow!("no satchel at {}", path.display())))?;
    serde_json::from_str(&raw).map_err(Error::from)
}

fn write_payload(dest: &Path, rel: &Path, bytes: &[u8]) -> Result<()> {
    let path = dest.join(rel);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(Error::from)?;
    }
    std::fs::write(&path, bytes).map_err(Error::from)
}

fn write_manifest(dest: &Path, payload: &[(PathBuf, String)]) -> Result<()> {
    let mut out = String::new();
    for (rel, hash) in payload {
        // Two spaces, the way every sha256sum file has them, so `sha256sum -c`
        // reads this without a translator.
        out.push_str(&format!("{hash}  {}\n", rel.display()));
    }
    std::fs::write(dest.join("manifest-sha256.txt"), out).map_err(Error::from)
}

fn write_declaration(dest: &Path, satchel: &Satchel) -> Result<()> {
    std::fs::write(
        dest.join("bagit.txt"),
        "BagIt-Version: 1.0\nTag-File-Character-Encoding: UTF-8\n",
    )
    .map_err(Error::from)?;
    let info = format!(
        "Bag-Software-Agent: vissue {}\nBagging-Date: {}\nSource-Organization: {}\nExternal-Description: {} issues and {} deed accessions from a vissue tracker\nInternal-Sender-Identifier: {}\n",
        env!("CARGO_PKG_VERSION"),
        satchel.packed_at,
        satchel.packed_by,
        satchel.issues.len(),
        satchel.needs.len(),
        VERSION,
    );
    std::fs::write(dest.join("bag-info.txt"), info).map_err(Error::from)
}

fn walk(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    if !dir.is_dir() {
        return Ok(out);
    }
    let mut stack = vec![dir.to_path_buf()];
    while let Some(at) = stack.pop() {
        for entry in std::fs::read_dir(&at).map_err(Error::from)? {
            let entry = entry.map_err(Error::from)?;
            let path = entry.path();
            if path.is_dir() {
                stack.push(path);
            } else {
                out.push(path);
            }
        }
    }
    out.sort();
    Ok(out)
}

fn digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    let hash = Sha256::digest(bytes);
    hash.iter().map(|b| format!("{b:02x}")).collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::DEFAULT_PREFIX;
    use crate::ops::{self, CreateOpts};

    /// `create` reports a line; the id is its first word.
    fn made(layout: &Layout, title: &str, opts: CreateOpts<'_>) -> String {
        let report = ops::create(layout, "sample", title, opts).expect("create");
        report
            .split_whitespace()
            .next()
            .expect("an id in the report")
            .to_string()
    }

    fn tracker() -> (tempfile::TempDir, Layout) {
        let dir = tempfile::tempdir().expect("tempdir");
        let layout = Layout::new(dir.path(), DEFAULT_PREFIX);
        std::fs::create_dir_all(layout.projects_dir()).expect("projects");
        (dir, layout)
    }

    /// A slice carries what it stands on, or the receiver has a task and no
    /// account of why it is not done.
    #[test]
    fn a_named_issue_brings_its_blockers_and_its_plan() {
        let (_dir, layout) = tracker();
        let plan = made(&layout, "the plan", CreateOpts::default());
        let blocker = made(&layout, "the blocker", CreateOpts::default());
        let work = made(
            &layout,
            "the work",
            CreateOpts {
                parent: Some(plan.as_str()),
                ..CreateOpts::default()
            },
        );
        ops::update(&layout, &work, None, None, Some(&blocker), None).expect("blocked by");
        let bystander = made(&layout, "not asked for", CreateOpts::default());

        let out = tempfile::tempdir().expect("out");
        let report = pack(
            &layout,
            &Slice {
                projects: Vec::new(),
                issues: vec![work.clone()],
            },
            out.path(),
        )
        .expect("packs");

        let described = describe(out.path()).expect("describes");
        assert!(described.issues.contains(&work), "{described:?}");
        assert!(described.issues.contains(&plan), "the plan is missing");
        assert!(
            described.issues.contains(&blocker),
            "the blocker is missing"
        );
        assert!(
            !described.issues.contains(&bystander),
            "a sibling came along uninvited"
        );
        // And the receiver is told which of them they did not ask for.
        assert_eq!(described.carried.len(), 2, "{:?}", described.carried);
        assert_eq!(report.issues, 3);
        assert!(
            out.path()
                .join("data/issues")
                .join(format!("{work}.org"))
                .is_file()
        );
    }

    /// A satchel checks out when it arrives whole, and says what is wrong when
    /// it does not.
    #[test]
    fn a_satchel_checks_out_until_something_moves() {
        let (_dir, layout) = tracker();
        let one = made(&layout, "first", CreateOpts::default());
        let out = tempfile::tempdir().expect("out");
        pack(
            &layout,
            &Slice {
                projects: vec!["sample".into()],
                issues: Vec::new(),
            },
            out.path(),
        )
        .expect("packs");

        verify(out.path()).expect("a fresh satchel checks out");

        // A payload file edited in transit.
        let issue = out.path().join("data/issues").join(format!("{one}.org"));
        std::fs::write(&issue, "* TODO something else\n").expect("write");
        let err = verify(out.path()).expect_err("an edited payload passed");
        assert!(format!("{err}").contains("changed"), "{err}");
    }

    /// The manifest has to account for the whole payload, not only for what it
    /// lists. Proving what you enumerated is how a bundle becomes a delivery
    /// mechanism for what you did not.
    #[test]
    fn a_file_nobody_listed_is_a_finding() {
        let (_dir, layout) = tracker();
        made(&layout, "first", CreateOpts::default());
        let out = tempfile::tempdir().expect("out");
        pack(
            &layout,
            &Slice {
                projects: vec!["sample".into()],
                issues: Vec::new(),
            },
            out.path(),
        )
        .expect("packs");
        verify(out.path()).expect("checks out");

        std::fs::write(out.path().join("data/extra.sh"), "rm -rf /\n").expect("write");
        let err = verify(out.path()).expect_err("an unlisted file passed");
        assert!(format!("{err}").contains("unlisted"), "{err}");
    }

    /// Sealing takes in what the deed store added, so the manifest covers the
    /// whole payload rather than only the half the tracker wrote.
    #[test]
    fn sealing_accounts_for_what_arrived_after_packing() {
        let (_dir, layout) = tracker();
        made(&layout, "first", CreateOpts::default());
        let out = tempfile::tempdir().expect("out");
        pack(
            &layout,
            &Slice {
                projects: vec!["sample".into()],
                issues: Vec::new(),
            },
            out.path(),
        )
        .expect("packs");

        // The deed store writes into the payload after the fact.
        let deeds = out.path().join("data/deeds/deed-file-note");
        std::fs::create_dir_all(&deeds).expect("mkdir");
        std::fs::write(deeds.join("deed.bin"), b"deed bytes").expect("write");

        // Before sealing that file is in the payload and not in the manifest,
        // which is exactly what verify is supposed to object to.
        let err = verify(out.path()).expect_err("an unsealed addition passed");
        assert!(format!("{err}").contains("unlisted"), "{err}");

        let report = seal(out.path()).expect("seals");
        assert!(report.files >= 3, "{report:?}");
        verify(out.path()).expect("a sealed satchel checks out");
    }

    /// Atoms are payload like anything else: the manifest covers them once
    /// sealed, and the check says they arrived.
    #[test]
    fn a_pack_can_put_what_the_seat_learned_in_too() {
        let (_dir, layout) = tracker();
        made(&layout, "first", CreateOpts::default());
        let out = tempfile::tempdir().expect("out");
        pack(
            &layout,
            &Slice {
                projects: vec!["sample".into()],
                issues: Vec::new(),
            },
            out.path(),
        )
        .expect("packs");

        // What `packset export --into` writes.
        let atoms = out.path().join("data/atoms");
        std::fs::create_dir_all(&atoms).expect("mkdir");
        std::fs::write(
            atoms.join("seat.jsonl"),
            "{\"id\":\"a1\",\"text\":\"what was learned\"}\n             {\"id\":\"a2\",\"text\":\"and this\"}\n",
        )
        .expect("write");

        let sealed = seal(out.path()).expect("seals");
        assert!(
            sealed.notes.iter().any(|n| n.contains("2 atoms")),
            "{:?}",
            sealed.notes
        );
        let checked = verify(out.path()).expect("checks out");
        assert!(
            checked.notes.iter().any(|n| n.contains("2 atoms")),
            "{:?}",
            checked.notes
        );

        // And an atom file added after sealing is unlisted, the same as any
        // other payload nobody agreed to.
        std::fs::write(atoms.join("late.jsonl"), "{\"id\":\"a3\"}\n").expect("write");
        let err = verify(out.path()).expect_err("a late atom file passed");
        assert!(format!("{err}").contains("unlisted"), "{err}");
    }

    /// A clean check says what it established and what it did not.
    ///
    /// The failure this guards is a reader running one command, seeing that
    /// the payload matches, and hearing that the bag is trustworthy. Matching
    /// a manifest a stranger could have written establishes nothing about who
    /// wrote it.
    #[test]
    fn checking_a_satchel_says_what_it_did_not_check() {
        let (_dir, layout) = tracker();
        made(&layout, "first", CreateOpts::default());
        let out = tempfile::tempdir().expect("out");
        pack(
            &layout,
            &Slice {
                projects: vec!["sample".into()],
                issues: Vec::new(),
            },
            out.path(),
        )
        .expect("packs");

        let unsigned = verify(out.path()).expect("checks out");
        let said = unsigned.notes.join(" ");
        assert!(
            said.contains("nothing about who wrote it"),
            "an unsigned satchel did not say so: {said}"
        );

        // With a signature beside the manifest, the check names the verb that
        // answers the other question rather than implying it answered it.
        std::fs::write(
            out.path().join("manifest-sha256.txt.sig"),
            "ed25519 aa bb\n",
        )
        .expect("write");
        let signed = verify(out.path()).expect("still checks out");
        let said = signed.notes.join(" ");
        assert!(said.contains("did not verify"), "{said}");
        assert!(said.contains("vouch check"), "{said}");
    }

    /// A deed that arrived is a deed nothing here has checked the provenance
    /// of, and the check says so rather than counting it as trust.
    ///
    /// The failure this guards is the same one as the signature note, one
    /// question further along: a reader who is told the bag matches its
    /// manifest and that three deeds arrived hears that the deeds are good.
    /// A manifest agrees just as well with a deed minted the morning of the
    /// handover.
    #[test]
    fn an_enclosed_deed_is_not_a_checked_deed() {
        let (_dir, layout) = tracker();
        made(&layout, "first", CreateOpts::default());
        let out = tempfile::tempdir().expect("out");
        pack(
            &layout,
            &Slice {
                projects: vec!["sample".into()],
                issues: Vec::new(),
            },
            out.path(),
        )
        .expect("packs");

        // A bag with no deeds says nothing about deeds.
        let bare = verify(out.path()).expect("checks out");
        assert!(
            !bare.notes.join(" ").contains("deeds arrived"),
            "{:?}",
            bare.notes
        );

        // The deed store's half arrives: one deed with the proof an export
        // writes, one without.
        let deeds = out.path().join("data").join("deeds");
        for (accession, proof) in [("deed-file-proven", true), ("deed-file-bare", false)] {
            let held = deeds.join(accession);
            std::fs::create_dir_all(&held).expect("dirs");
            std::fs::write(held.join("deed.bin"), b"bytes").expect("bytes");
            if proof {
                std::fs::write(
                    held.join("proof.txt"),
                    "id=deed-file-proven
",
                )
                .expect("proof");
            }
        }
        seal(out.path()).expect("seals");

        let mixed = verify(out.path()).expect("checks out");
        let said = mixed.notes.join(" ");
        assert!(said.contains("deed-file-bare"), "{said}");
        assert!(
            said.contains("nothing says they were logged"),
            "a deed with no proof passed unremarked: {said}"
        );
        assert!(
            !said.contains("deed-file-proven"),
            "a deed carrying a proof was named as missing one: {said}"
        );

        // With every deed carrying one, the note names the verb that reads
        // them rather than implying this check did.
        std::fs::write(
            deeds.join("deed-file-bare").join("proof.txt"),
            "id=deed-file-bare
",
        )
        .expect("proof");
        seal(out.path()).expect("seals");
        let whole = verify(out.path()).expect("checks out");
        let said = whole.notes.join(" ");
        assert!(said.contains("2 deeds arrived"), "{said}");
        assert!(said.contains("deedar check"), "{said}");
    }

    /// A slice that names nothing is not a slice, and an issue that is not
    /// there is not packed silently.
    #[test]
    fn an_empty_or_unknown_slice_is_refused() {
        let (_dir, layout) = tracker();
        let out = tempfile::tempdir().expect("out");
        assert!(pack(&layout, &Slice::default(), out.path()).is_err());
        assert!(
            pack(
                &layout,
                &Slice {
                    projects: Vec::new(),
                    issues: vec!["sample-nope".into()],
                },
                out.path(),
            )
            .is_err()
        );
    }
}
