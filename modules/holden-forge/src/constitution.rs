//! Agent constitutions derived from a pack (`methodology/LW-AGENT-FORGE.md`).
//!
//! One derivation builds a Constitution IR; [`render`] writes it as the
//! one-shot Form A text and [`parse`] reads that text back into the same IR.
//! The nine sections and their order follow the derivation map (§2). Every
//! entry carries the JSON pointer of the pack field it derives from, so a
//! reader can check each line against the pack.

use crate::pack::{Item, LoadedPack, Pack, Role};

/// The derivation map's sections, in order, each with the fixed sentence
/// that opens it. Titles match `methodology/LW-AGENT-FORGE.md` §2.
pub const SECTIONS: [(&str, &str); 9] = [
    (
        "IDENTITY & BINDING",
        "Who you are and what binds you. An earlier priority entry outranks a later one.",
    ),
    (
        "DOMAIN MAP",
        "Every super-domain and category of the pack with its seam. Work marked not-yours is outside your role: refuse it and name the seam.",
    ),
    (
        "ROLE LENSES",
        "The practitioner perspectives you adopt, and when each one applies.",
    ),
    (
        "OPERATING RULES",
        "Do and never-do rules. Each cites the pack item it comes from.",
    ),
    (
        "SELF-QC PROTOCOL",
        "Before you answer, grade your own output: ask each question, apply its pass condition, and tick its checklist line.",
    ),
    (
        "TASK ROUTING",
        "Take a task when an applies signal matches it. Refuse it when a refuse signal matches, and name the item.",
    ),
    (
        "FAILURE-MODE REGISTRY",
        "The known ways to be confidently wrong in this role, each with its guard.",
    ),
    ("CROSS-CUTTING LAWS", "Always-on invariants for every task."),
    (
        "EVIDENCE DISCIPLINE",
        "Laws every constitution the forge emits inherits, whatever the domain.",
    ),
];

const BANNER: [&str; 2] = [
    "> Emitted by the forge from one domain pack. Do not hand-edit: re-emit when the pack digest changes.",
    "> Every entry ends with the pack field it derives from.",
];

/// The constitution text format version.
pub const FORMAT: &str = "1";

/// One line of a constitution section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    /// A short tag: an item citation such as `03.01`, or a key such as `pack`.
    pub tag: String,
    /// The text, equal to the pack value at `trace`.
    pub text: String,
    /// JSON pointer into the pack.
    pub trace: String,
}

/// A constitution section.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Section {
    /// One of [`SECTIONS`].
    pub title: String,
    /// Its entries.
    pub entries: Vec<Entry>,
}

/// The Constitution IR.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Constitution {
    /// The role.
    pub role: Role,
    /// The pack name.
    pub pack_name: String,
    /// `sha256:<hex>` of the pack file.
    pub pack_digest: String,
    /// The pack's ratification status.
    pub ratification: String,
    /// The nine sections, in derivation-map order.
    pub sections: Vec<Section>,
}

fn entry(tag: impl Into<String>, text: &str, trace: String) -> Entry {
    Entry {
        tag: tag.into(),
        text: text.to_owned(),
        trace,
    }
}

/// Items of the super-domains a role owns, with their JSON pointer and
/// citation (`CC.II`).
fn focus_items<'a>(pack: &'a Pack, codes: &[String]) -> Vec<(String, String, &'a Item)> {
    let mut out = Vec::new();
    for (si, sd) in pack.super_domains.iter().enumerate() {
        if !codes.contains(&sd.code) {
            continue;
        }
        for (ci, c) in sd.categories.iter().enumerate() {
            for (ii, it) in c.items.iter().enumerate() {
                out.push((
                    format!("/super_domains/{si}/categories/{ci}/items/{ii}"),
                    format!("{}.{}", c.code, it.item_code),
                    it,
                ));
            }
        }
    }
    out
}

/// Derive the constitution of `role` from a validated pack.
pub fn derive(loaded: &LoadedPack, role: Role) -> Result<Constitution, String> {
    let pack = &loaded.pack;
    let (ri, focus) = pack
        .role_focus
        .iter()
        .enumerate()
        .find(|(_, r)| r.role == role)
        .ok_or_else(|| format!("the pack has no cut for role '{}'", role.as_str()))?;
    let items = focus_items(pack, &focus.super_domains);
    let mut sections: Vec<Vec<Entry>> = vec![Vec::new(); SECTIONS.len()];

    // 1. IDENTITY & BINDING <- pack manifest + the role cut.
    let id = &mut sections[0];
    id.push(entry(
        "role",
        &focus.identity,
        format!("/role_focus/{ri}/identity"),
    ));
    id.push(entry("pack", &pack.pack_name, "/pack_name".into()));
    id.push(entry("domain", &pack.domain, "/domain".into()));
    id.push(entry(
        "ratification",
        &pack.ratification_status,
        "/ratification_status".into(),
    ));
    for (i, p) in pack.priority_order.iter().enumerate() {
        id.push(entry(
            format!("priority-{}", i + 1),
            p,
            format!("/priority_order/{i}"),
        ));
    }

    // 2. DOMAIN MAP <- super-domains + categories + seams.
    let map = &mut sections[1];
    for (si, sd) in pack.super_domains.iter().enumerate() {
        let mine = focus.super_domains.contains(&sd.code);
        let mark = if mine { "" } else { ":not-yours" };
        map.push(entry(
            format!("{}{mark}", sd.code),
            &sd.name,
            format!("/super_domains/{si}/name"),
        ));
        for (ci, c) in sd.categories.iter().enumerate() {
            let b = format!("/super_domains/{si}/categories/{ci}");
            map.push(entry(
                format!("{}.{}{mark}", sd.code, c.code),
                &c.name,
                format!("{b}/name"),
            ));
            map.push(entry(
                format!("{}.{}:seam", sd.code, c.code),
                &c.seam,
                format!("{b}/seam"),
            ));
        }
    }

    // 3. ROLE LENSES <- worldviews named by the role cut.
    for wid in &focus.worldviews {
        if let Some((wi, w)) = pack
            .worldviews
            .iter()
            .enumerate()
            .find(|(_, w)| &w.id == wid)
        {
            sections[2].push(entry(
                w.id.clone(),
                &w.name,
                format!("/worldviews/{wi}/name"),
            ));
            sections[2].push(entry(
                format!("{}:when", w.id),
                &w.applies_when,
                format!("/worldviews/{wi}/applies_when"),
            ));
        }
    }

    for (base, cite, it) in &items {
        // 4. OPERATING RULES <- synthesis_instructions[].
        for (k, s) in it.synthesis_instructions.iter().enumerate() {
            sections[3].push(entry(
                cite.clone(),
                s,
                format!("{base}/synthesis_instructions/{k}"),
            ));
        }
        // 5. SELF-QC PROTOCOL <- internal_evaluation_criteria[].
        for (k, c) in it.internal_evaluation_criteria.iter().enumerate() {
            let b = format!("{base}/internal_evaluation_criteria/{k}");
            sections[4].push(entry(
                format!("{}:question", c.criterion_id),
                &c.question,
                format!("{b}/question"),
            ));
            sections[4].push(entry(
                format!("{}:pass", c.criterion_id),
                &c.pass_condition,
                format!("{b}/pass_condition"),
            ));
            sections[4].push(entry(
                format!("{}:check", c.criterion_id),
                &c.checklist,
                format!("{b}/checklist"),
            ));
        }
        // 6. TASK ROUTING <- applicability_signals / anti_signals.
        for (k, s) in it.applicability_signals.iter().enumerate() {
            sections[5].push(entry(
                format!("{cite}:applies"),
                s,
                format!("{base}/applicability_signals/{k}"),
            ));
        }
        for (k, s) in it.anti_signals.iter().enumerate() {
            sections[5].push(entry(
                format!("{cite}:refuse"),
                s,
                format!("{base}/anti_signals/{k}"),
            ));
        }
        // 7. FAILURE-MODE REGISTRY <- signature failures.
        for (k, f) in it.signature_failures.iter().enumerate() {
            let b = format!("{base}/signature_failures/{k}");
            sections[6].push(entry(cite.clone(), &f.failure, format!("{b}/failure")));
            sections[6].push(entry(
                format!("{cite}:guard"),
                &f.guard,
                format!("{b}/guard"),
            ));
        }
    }

    // 8. CROSS-CUTTING LAWS <- cross_cutting_postures.
    for (i, x) in pack.cross_cutting_postures.iter().enumerate() {
        sections[7].push(entry(
            x.id.clone(),
            &x.name,
            format!("/cross_cutting_postures/{i}/name"),
        ));
        sections[7].push(entry(
            format!("{}:law", x.id),
            &x.posture,
            format!("/cross_cutting_postures/{i}/posture"),
        ));
    }

    // 9. EVIDENCE DISCIPLINE <- module laws.
    for (i, l) in pack.module_laws.iter().enumerate() {
        sections[8].push(entry(l.id.clone(), &l.law, format!("/module_laws/{i}/law")));
    }

    Ok(Constitution {
        role,
        pack_name: pack.pack_name.clone(),
        pack_digest: loaded.digest.clone(),
        ratification: pack.ratification_status.clone(),
        sections: SECTIONS
            .iter()
            .zip(sections)
            .map(|((title, _), entries)| Section {
                title: (*title).to_owned(),
                entries,
            })
            .collect(),
    })
}

fn intro(title: &str) -> Option<&'static str> {
    SECTIONS.iter().find(|(t, _)| *t == title).map(|(_, i)| *i)
}

/// Render the constitution as Form A text.
pub fn render(c: &Constitution) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# Agent constitution: {} / {}\n\n",
        c.pack_name,
        c.role.as_str()
    ));
    for b in BANNER {
        out.push_str(b);
        out.push('\n');
    }
    out.push('\n');
    out.push_str(&format!("- forge-constitution: {FORMAT}\n"));
    out.push_str(&format!("- role: {}\n", c.role.as_str()));
    out.push_str(&format!("- pack: {}\n", c.pack_name));
    out.push_str(&format!("- pack_digest: {}\n", c.pack_digest));
    out.push_str(&format!("- ratification: {}\n", c.ratification));
    for (n, s) in c.sections.iter().enumerate() {
        out.push_str(&format!("\n## {}. {}\n\n", n + 1, s.title));
        out.push_str(intro(&s.title).unwrap_or(""));
        out.push_str("\n\n");
        for e in &s.entries {
            out.push_str(&format!("- [{}] {} [trace: {}]\n", e.tag, e.text, e.trace));
        }
    }
    out
}

fn header<'a>(line: Option<&'a str>, key: &str) -> Result<&'a str, String> {
    line.and_then(|l| l.strip_prefix("- "))
        .and_then(|l| l.strip_prefix(key))
        .and_then(|l| l.strip_prefix(": "))
        .ok_or_else(|| format!("expected the header line '- {key}: ...'"))
}

fn parse_entry(line: &str) -> Result<Entry, String> {
    let bad = || format!("malformed entry line: {line}");
    let rest = line.strip_prefix("- [").ok_or_else(bad)?;
    let (tag, rest) = rest.split_once("] ").ok_or_else(bad)?;
    let (text, trace) = rest.rsplit_once(" [trace: ").ok_or_else(bad)?;
    let trace = trace.strip_suffix(']').ok_or_else(bad)?;
    if tag.is_empty() || text.is_empty() || !trace.starts_with('/') {
        return Err(bad());
    }
    Ok(Entry {
        tag: tag.to_owned(),
        text: text.to_owned(),
        trace: trace.to_owned(),
    })
}

/// Read a Form A constitution back into the IR. The text must be exactly
/// what [`render`] writes for that IR; anything else is refused.
pub fn parse(text: &str) -> Result<Constitution, String> {
    let mut lines = text.lines();
    let title = lines
        .next()
        .and_then(|l| l.strip_prefix("# Agent constitution: "))
        .ok_or("missing the title line")?;
    let (_, title_role) = title.rsplit_once(" / ").ok_or("malformed title line")?;
    if lines.next() != Some("") {
        return Err("expected a blank line after the title".into());
    }
    for b in BANNER {
        if lines.next() != Some(b) {
            return Err("the banner differs from the forge's banner".into());
        }
    }
    if lines.next() != Some("") {
        return Err("expected a blank line after the banner".into());
    }
    if header(lines.next(), "forge-constitution")? != FORMAT {
        return Err("unsupported constitution format".into());
    }
    let role_name = header(lines.next(), "role")?;
    let role = Role::parse(role_name).ok_or_else(|| format!("unknown role '{role_name}'"))?;
    if role_name != title_role {
        return Err("the title role and the header role differ".into());
    }
    let pack_name = header(lines.next(), "pack")?.to_owned();
    let pack_digest = header(lines.next(), "pack_digest")?.to_owned();
    let ratification = header(lines.next(), "ratification")?.to_owned();

    let mut sections: Vec<Section> = Vec::new();
    let mut current: Option<Section> = None;
    let mut expect_intro = false;
    for line in lines {
        if let Some(h) = line.strip_prefix("## ") {
            if let Some(s) = current.take() {
                sections.push(s);
            }
            let (num, title) = h.split_once(". ").ok_or("malformed section heading")?;
            if num != (sections.len() + 1).to_string() {
                return Err(format!("section '{title}' is out of order"));
            }
            if intro(title).is_none() {
                return Err(format!("unknown section '{title}'"));
            }
            current = Some(Section {
                title: title.to_owned(),
                entries: Vec::new(),
            });
            expect_intro = true;
        } else if line.is_empty() {
            continue;
        } else if expect_intro {
            let s = current.as_ref().ok_or("text before the first section")?;
            if Some(line) != intro(&s.title) {
                return Err(format!("the opening sentence of '{}' differs", s.title));
            }
            expect_intro = false;
        } else {
            let s = current.as_mut().ok_or("entry before the first section")?;
            s.entries.push(parse_entry(line)?);
        }
    }
    if let Some(s) = current.take() {
        sections.push(s);
    }
    let c = Constitution {
        role,
        pack_name,
        pack_digest,
        ratification,
        sections,
    };
    if render(&c) != text {
        return Err("the text is not in the forge's canonical layout".into());
    }
    Ok(c)
}
