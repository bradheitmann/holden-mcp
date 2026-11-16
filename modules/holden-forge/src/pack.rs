//! The typed domain pack (the forge's schema) and its reader and writer.
//!
//! A pack is one JSON file in canonical layout. It carries the CCII depth
//! contract of `methodology/LW-DOMAIN-FORGE.md` §0 per item, plus the inputs
//! `methodology/LW-AGENT-FORGE.md` §2 derives constitutions from: worldviews,
//! cross-cutting postures, module laws, and the role cuts (`role_focus`, §7).
//!
//! The reader is strict: an unknown member, a missing member or a wrong type
//! is refused and named by its JSON pointer. [`Pack::to_value`] writes the
//! members back in schema order, so `read -> write` is lossless for every
//! valid pack file.

use crate::json::{self, Value};
use crate::sha256;

/// The pack schema id this forge reads.
pub const PACK_SCHEMA: &str = "holden-forge.pack/1";

/// The four holden agent roles.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Role {
    /// Turns owner promises and planning evidence into situated cognitions.
    Planner,
    /// Seals the how and guards the moment-of-test secret.
    Sealer,
    /// Runs holdouts at the product boundary and records verdicts.
    Evaluator,
    /// Attacks the seal, the holdouts and the evaluator.
    Adversary,
}

impl Role {
    /// Every role, in a fixed order.
    pub const ALL: [Role; 4] = [
        Role::Planner,
        Role::Sealer,
        Role::Evaluator,
        Role::Adversary,
    ];

    /// The role's name as written in packs and on the command line.
    pub fn as_str(self) -> &'static str {
        match self {
            Role::Planner => "planner",
            Role::Sealer => "sealer",
            Role::Evaluator => "evaluator",
            Role::Adversary => "adversary",
        }
    }

    /// Parse a role name.
    pub fn parse(s: &str) -> Option<Role> {
        Role::ALL.into_iter().find(|r| r.as_str() == s)
    }
}

/// A practitioner perspective (input manifest I5).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Worldview {
    /// Stable id, e.g. `W1`.
    pub id: String,
    /// Short name.
    pub name: String,
    /// When an agent adopts this lens.
    pub applies_when: String,
}

/// One falsifiable evaluation criterion of an item.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Criterion {
    /// `<item_id>-C<n>`.
    pub criterion_id: String,
    /// The falsifiable question.
    pub question: String,
    /// What passing means.
    pub pass_condition: String,
    /// The reviewer's checklist line.
    pub checklist: String,
}

/// A signature failure the item guards against.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Failure {
    /// The way to be confidently wrong.
    pub failure: String,
    /// The guard against it.
    pub guard: String,
}

/// One item: the CCII depth contract.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Item {
    /// `<category code><item code>`, four digits.
    pub item_id: String,
    /// Two-digit item code inside the category.
    pub item_code: String,
    /// Item name.
    pub item_name: String,
    /// The reasoning paragraph: coverage, why it matters, seams.
    pub detailed_description: String,
    /// Operational capture and refuse rules.
    pub synthesis_instructions: Vec<String>,
    /// The grading instrument.
    pub internal_evaluation_criteria: Vec<Criterion>,
    /// When the item applies.
    pub applicability_signals: Vec<String>,
    /// When it does not.
    pub anti_signals: Vec<String>,
    /// Signature failures and their guards.
    pub signature_failures: Vec<Failure>,
}

/// A category under a super-domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Category {
    /// Two-digit category code.
    pub code: String,
    /// Category name.
    pub name: String,
    /// One-line seam against the nearest neighbour category.
    pub seam: String,
    /// Items.
    pub items: Vec<Item>,
}

/// A super-domain.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SuperDomain {
    /// One-letter code.
    pub code: String,
    /// Name.
    pub name: String,
    /// Categories.
    pub categories: Vec<Category>,
}

/// An always-on cross-cutting posture.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Posture {
    /// Stable id, e.g. `X1`.
    pub id: String,
    /// Name.
    pub name: String,
    /// The invariant.
    pub posture: String,
}

/// A domain-independent module law (evidence discipline).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Law {
    /// Stable id, e.g. `L1`.
    pub id: String,
    /// The law.
    pub law: String,
}

/// A role cut of the pack (`role_focus`, LW-AGENT-FORGE §7).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RoleFocus {
    /// The role.
    pub role: Role,
    /// Who the agent is, in one sentence.
    pub identity: String,
    /// Super-domain codes this role owns.
    pub super_domains: Vec<String>,
    /// Worldview ids this role adopts.
    pub worldviews: Vec<String>,
}

/// A domain pack.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Pack {
    /// Always [`PACK_SCHEMA`].
    pub schema: String,
    /// Pack name (slug).
    pub pack_name: String,
    /// The domain, in one line.
    pub domain: String,
    /// `DRAFT` until the owner ratifies.
    pub ratification_status: String,
    /// The priority order every constitution states.
    pub priority_order: Vec<String>,
    /// Practitioner perspectives.
    pub worldviews: Vec<Worldview>,
    /// The categorical spine.
    pub super_domains: Vec<SuperDomain>,
    /// Always-on invariants.
    pub cross_cutting_postures: Vec<Posture>,
    /// Inherited evidence laws.
    pub module_laws: Vec<Law>,
    /// One cut per role.
    pub role_focus: Vec<RoleFocus>,
}

/// A pack error, named by JSON pointer.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PackError {
    /// Where.
    pub path: String,
    /// What.
    pub message: String,
}

impl std::fmt::Display for PackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let path = if self.path.is_empty() {
            "/"
        } else {
            &self.path
        };
        write!(f, "{path}: {}", self.message)
    }
}

fn perr<T>(path: &str, message: impl Into<String>) -> Result<T, PackError> {
    Err(PackError {
        path: path.to_owned(),
        message: message.into(),
    })
}

/// Reads one object, refusing unknown and missing members.
struct Obj<'a> {
    path: String,
    members: &'a [(String, Value)],
    expected: &'static [&'static str],
}

impl<'a> Obj<'a> {
    fn new(v: &'a Value, path: &str, expected: &'static [&'static str]) -> Result<Self, PackError> {
        let Value::Object(members) = v else {
            return perr(path, "expected an object");
        };
        let keys: Vec<&str> = members.iter().map(|(k, _)| k.as_str()).collect();
        for k in &keys {
            if !expected.contains(k) {
                return perr(&format!("{path}/{k}"), "unknown member");
            }
        }
        if keys != expected {
            for e in expected {
                if !keys.contains(e) {
                    return perr(&format!("{path}/{e}"), "missing member");
                }
            }
            return perr(
                path,
                format!(
                    "members out of schema order (expected {})",
                    expected.join(", ")
                ),
            );
        }
        Ok(Obj {
            path: path.to_owned(),
            members,
            expected,
        })
    }

    fn field(&self, key: &str) -> (&'a Value, String) {
        let v = self
            .members
            .iter()
            .find(|(k, _)| k == key)
            .map_or(&Value::Null, |(_, v)| v);
        debug_assert!(self.expected.contains(&key));
        (v, format!("{}/{key}", self.path))
    }

    fn string(&self, key: &str) -> Result<String, PackError> {
        let (v, path) = self.field(key);
        read_string(v, &path)
    }

    fn strings(&self, key: &str) -> Result<Vec<String>, PackError> {
        let (v, path) = self.field(key);
        read_array(v, &path, read_string)
    }

    fn array<T>(
        &self,
        key: &str,
        each: impl Fn(&Value, &str) -> Result<T, PackError>,
    ) -> Result<Vec<T>, PackError> {
        let (v, path) = self.field(key);
        read_array(v, &path, each)
    }
}

fn read_string(v: &Value, path: &str) -> Result<String, PackError> {
    match v {
        Value::String(s) => Ok(s.clone()),
        _ => perr(path, "expected a string"),
    }
}

fn read_array<T>(
    v: &Value,
    path: &str,
    each: impl Fn(&Value, &str) -> Result<T, PackError>,
) -> Result<Vec<T>, PackError> {
    let Value::Array(items) = v else {
        return perr(path, "expected an array");
    };
    items
        .iter()
        .enumerate()
        .map(|(i, item)| each(item, &format!("{path}/{i}")))
        .collect()
}

const PACK_KEYS: &[&str] = &[
    "schema",
    "pack_name",
    "domain",
    "ratification_status",
    "priority_order",
    "worldviews",
    "super_domains",
    "cross_cutting_postures",
    "module_laws",
    "role_focus",
];
const WORLDVIEW_KEYS: &[&str] = &["id", "name", "applies_when"];
const SUPER_KEYS: &[&str] = &["code", "name", "categories"];
const CATEGORY_KEYS: &[&str] = &["code", "name", "seam", "items"];
const ITEM_KEYS: &[&str] = &[
    "item_id",
    "item_code",
    "item_name",
    "detailed_description",
    "synthesis_instructions",
    "internal_evaluation_criteria",
    "applicability_signals",
    "anti_signals",
    "signature_failures",
];
const CRITERION_KEYS: &[&str] = &["criterion_id", "question", "pass_condition", "checklist"];
const FAILURE_KEYS: &[&str] = &["failure", "guard"];
const POSTURE_KEYS: &[&str] = &["id", "name", "posture"];
const LAW_KEYS: &[&str] = &["id", "law"];
const ROLE_KEYS: &[&str] = &["role", "identity", "super_domains", "worldviews"];

fn read_item(v: &Value, path: &str) -> Result<Item, PackError> {
    let o = Obj::new(v, path, ITEM_KEYS)?;
    Ok(Item {
        item_id: o.string("item_id")?,
        item_code: o.string("item_code")?,
        item_name: o.string("item_name")?,
        detailed_description: o.string("detailed_description")?,
        synthesis_instructions: o.strings("synthesis_instructions")?,
        internal_evaluation_criteria: o.array("internal_evaluation_criteria", |v, p| {
            let c = Obj::new(v, p, CRITERION_KEYS)?;
            Ok(Criterion {
                criterion_id: c.string("criterion_id")?,
                question: c.string("question")?,
                pass_condition: c.string("pass_condition")?,
                checklist: c.string("checklist")?,
            })
        })?,
        applicability_signals: o.strings("applicability_signals")?,
        anti_signals: o.strings("anti_signals")?,
        signature_failures: o.array("signature_failures", |v, p| {
            let f = Obj::new(v, p, FAILURE_KEYS)?;
            Ok(Failure {
                failure: f.string("failure")?,
                guard: f.string("guard")?,
            })
        })?,
    })
}

impl Pack {
    /// Read a pack from its JSON value (typed, strict).
    pub fn from_value(v: &Value) -> Result<Pack, PackError> {
        let o = Obj::new(v, "", PACK_KEYS)?;
        Ok(Pack {
            schema: o.string("schema")?,
            pack_name: o.string("pack_name")?,
            domain: o.string("domain")?,
            ratification_status: o.string("ratification_status")?,
            priority_order: o.strings("priority_order")?,
            worldviews: o.array("worldviews", |v, p| {
                let w = Obj::new(v, p, WORLDVIEW_KEYS)?;
                Ok(Worldview {
                    id: w.string("id")?,
                    name: w.string("name")?,
                    applies_when: w.string("applies_when")?,
                })
            })?,
            super_domains: o.array("super_domains", |v, p| {
                let s = Obj::new(v, p, SUPER_KEYS)?;
                Ok(SuperDomain {
                    code: s.string("code")?,
                    name: s.string("name")?,
                    categories: s.array("categories", |v, p| {
                        let c = Obj::new(v, p, CATEGORY_KEYS)?;
                        Ok(Category {
                            code: c.string("code")?,
                            name: c.string("name")?,
                            seam: c.string("seam")?,
                            items: c.array("items", read_item)?,
                        })
                    })?,
                })
            })?,
            cross_cutting_postures: o.array("cross_cutting_postures", |v, p| {
                let x = Obj::new(v, p, POSTURE_KEYS)?;
                Ok(Posture {
                    id: x.string("id")?,
                    name: x.string("name")?,
                    posture: x.string("posture")?,
                })
            })?,
            module_laws: o.array("module_laws", |v, p| {
                let l = Obj::new(v, p, LAW_KEYS)?;
                Ok(Law {
                    id: l.string("id")?,
                    law: l.string("law")?,
                })
            })?,
            role_focus: o.array("role_focus", |v, p| {
                let r = Obj::new(v, p, ROLE_KEYS)?;
                let name = r.string("role")?;
                let Some(role) = Role::parse(&name) else {
                    return perr(&format!("{p}/role"), format!("unknown role '{name}'"));
                };
                Ok(RoleFocus {
                    role,
                    identity: r.string("identity")?,
                    super_domains: r.strings("super_domains")?,
                    worldviews: r.strings("worldviews")?,
                })
            })?,
        })
    }

    /// The pack as a JSON value, members in schema order.
    pub fn to_value(&self) -> Value {
        fn s(v: &str) -> Value {
            Value::String(v.to_owned())
        }
        fn ss(v: &[String]) -> Value {
            Value::Array(v.iter().map(|x| s(x)).collect())
        }
        fn obj(members: Vec<(&str, Value)>) -> Value {
            Value::Object(
                members
                    .into_iter()
                    .map(|(k, v)| (k.to_owned(), v))
                    .collect(),
            )
        }
        let item = |i: &Item| {
            obj(vec![
                ("item_id", s(&i.item_id)),
                ("item_code", s(&i.item_code)),
                ("item_name", s(&i.item_name)),
                ("detailed_description", s(&i.detailed_description)),
                ("synthesis_instructions", ss(&i.synthesis_instructions)),
                (
                    "internal_evaluation_criteria",
                    Value::Array(
                        i.internal_evaluation_criteria
                            .iter()
                            .map(|c| {
                                obj(vec![
                                    ("criterion_id", s(&c.criterion_id)),
                                    ("question", s(&c.question)),
                                    ("pass_condition", s(&c.pass_condition)),
                                    ("checklist", s(&c.checklist)),
                                ])
                            })
                            .collect(),
                    ),
                ),
                ("applicability_signals", ss(&i.applicability_signals)),
                ("anti_signals", ss(&i.anti_signals)),
                (
                    "signature_failures",
                    Value::Array(
                        i.signature_failures
                            .iter()
                            .map(|f| obj(vec![("failure", s(&f.failure)), ("guard", s(&f.guard))]))
                            .collect(),
                    ),
                ),
            ])
        };
        obj(vec![
            ("schema", s(&self.schema)),
            ("pack_name", s(&self.pack_name)),
            ("domain", s(&self.domain)),
            ("ratification_status", s(&self.ratification_status)),
            ("priority_order", ss(&self.priority_order)),
            (
                "worldviews",
                Value::Array(
                    self.worldviews
                        .iter()
                        .map(|w| {
                            obj(vec![
                                ("id", s(&w.id)),
                                ("name", s(&w.name)),
                                ("applies_when", s(&w.applies_when)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "super_domains",
                Value::Array(
                    self.super_domains
                        .iter()
                        .map(|sd| {
                            obj(vec![
                                ("code", s(&sd.code)),
                                ("name", s(&sd.name)),
                                (
                                    "categories",
                                    Value::Array(
                                        sd.categories
                                            .iter()
                                            .map(|c| {
                                                obj(vec![
                                                    ("code", s(&c.code)),
                                                    ("name", s(&c.name)),
                                                    ("seam", s(&c.seam)),
                                                    (
                                                        "items",
                                                        Value::Array(
                                                            c.items.iter().map(item).collect(),
                                                        ),
                                                    ),
                                                ])
                                            })
                                            .collect(),
                                    ),
                                ),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "cross_cutting_postures",
                Value::Array(
                    self.cross_cutting_postures
                        .iter()
                        .map(|x| {
                            obj(vec![
                                ("id", s(&x.id)),
                                ("name", s(&x.name)),
                                ("posture", s(&x.posture)),
                            ])
                        })
                        .collect(),
                ),
            ),
            (
                "module_laws",
                Value::Array(
                    self.module_laws
                        .iter()
                        .map(|l| obj(vec![("id", s(&l.id)), ("law", s(&l.law))]))
                        .collect(),
                ),
            ),
            (
                "role_focus",
                Value::Array(
                    self.role_focus
                        .iter()
                        .map(|r| {
                            obj(vec![
                                ("role", s(r.role.as_str())),
                                ("identity", s(&r.identity)),
                                ("super_domains", ss(&r.super_domains)),
                                ("worldviews", ss(&r.worldviews)),
                            ])
                        })
                        .collect(),
                ),
            ),
        ])
    }

    /// The canonical file text of this pack.
    pub fn to_canonical(&self) -> String {
        json::to_canonical(&self.to_value())
    }

    /// The role cut for `role`, if the pack has one.
    pub fn focus(&self, role: Role) -> Option<&RoleFocus> {
        self.role_focus.iter().find(|r| r.role == role)
    }

    /// Number of categories.
    pub fn category_count(&self) -> usize {
        self.super_domains.iter().map(|s| s.categories.len()).sum()
    }

    /// Number of items.
    pub fn item_count(&self) -> usize {
        self.super_domains
            .iter()
            .flat_map(|s| &s.categories)
            .map(|c| c.items.len())
            .sum()
    }
}

/// The digest that binds a constitution to its pack: `sha256:<hex>` of the
/// pack file bytes (the parentage law, LW-AGENT-FORGE header).
pub fn pack_digest(bytes: &[u8]) -> String {
    format!("sha256:{}", sha256::hex(bytes))
}

fn is_code(s: &str, len: usize) -> bool {
    s.len() == len && s.bytes().all(|b| b.is_ascii_digit())
}

fn is_id(s: &str) -> bool {
    !s.is_empty()
        && s.bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_')
}

/// Text that a constitution line can carry without ambiguity.
fn check_text(path: &str, s: &str, errors: &mut Vec<PackError>) {
    if s.trim().is_empty() {
        errors.push(PackError {
            path: path.to_owned(),
            message: "empty text".to_owned(),
        });
    } else if s.trim() != s || s.contains('\n') || s.contains('\r') || s.contains(" [trace: ") {
        errors.push(PackError {
            path: path.to_owned(),
            message: "text must be one trimmed line without ' [trace: '".to_owned(),
        });
    }
}

fn nonempty<T>(path: &str, v: &[T], errors: &mut Vec<PackError>) {
    if v.is_empty() {
        errors.push(PackError {
            path: path.to_owned(),
            message: "must not be empty".to_owned(),
        });
    }
}

/// Validate a pack against the forge's schema rules beyond types: ids,
/// codes, the depth contract (every list non-empty), unique ids, and role
/// cuts that name existing super-domains and worldviews. Returns every
/// error, each named by JSON pointer.
pub fn validate(p: &Pack) -> Vec<PackError> {
    let mut e = Vec::new();
    let mut push = |path: String, message: &str| {
        e.push(PackError {
            path,
            message: message.to_owned(),
        });
    };
    if p.schema != PACK_SCHEMA {
        push("/schema".into(), "unsupported pack schema");
    }
    if !is_id(&p.pack_name) {
        push("/pack_name".into(), "pack_name must be a slug");
    }
    let mut errors = e;
    check_text("/domain", &p.domain, &mut errors);
    check_text("/ratification_status", &p.ratification_status, &mut errors);
    nonempty("/priority_order", &p.priority_order, &mut errors);
    for (i, s) in p.priority_order.iter().enumerate() {
        check_text(&format!("/priority_order/{i}"), s, &mut errors);
    }
    let mut seen: Vec<String> = Vec::new();
    let mut unique = |path: String, id: &str, errors: &mut Vec<PackError>| {
        if !is_id(id) {
            errors.push(PackError {
                path,
                message: "id must be [A-Za-z0-9_-]+".to_owned(),
            });
        } else if seen.iter().any(|s| s == id) {
            errors.push(PackError {
                path,
                message: format!("duplicate id '{id}'"),
            });
        } else {
            seen.push(id.to_owned());
        }
    };
    nonempty("/worldviews", &p.worldviews, &mut errors);
    for (i, w) in p.worldviews.iter().enumerate() {
        let b = format!("/worldviews/{i}");
        unique(format!("{b}/id"), &w.id, &mut errors);
        check_text(&format!("{b}/name"), &w.name, &mut errors);
        check_text(&format!("{b}/applies_when"), &w.applies_when, &mut errors);
    }
    nonempty("/super_domains", &p.super_domains, &mut errors);
    for (si, sd) in p.super_domains.iter().enumerate() {
        let b = format!("/super_domains/{si}");
        if sd.code.len() != 1 || !sd.code.bytes().all(|c| c.is_ascii_uppercase()) {
            errors.push(PackError {
                path: format!("{b}/code"),
                message: "super-domain code must be one uppercase letter".to_owned(),
            });
        }
        unique(format!("{b}/code"), &sd.code, &mut errors);
        check_text(&format!("{b}/name"), &sd.name, &mut errors);
        nonempty(&format!("{b}/categories"), &sd.categories, &mut errors);
        for (ci, c) in sd.categories.iter().enumerate() {
            let cb = format!("{b}/categories/{ci}");
            if !is_code(&c.code, 2) {
                errors.push(PackError {
                    path: format!("{cb}/code"),
                    message: "category code must be two digits".to_owned(),
                });
            }
            unique(format!("{cb}/code"), &c.code, &mut errors);
            check_text(&format!("{cb}/name"), &c.name, &mut errors);
            check_text(&format!("{cb}/seam"), &c.seam, &mut errors);
            nonempty(&format!("{cb}/items"), &c.items, &mut errors);
            for (ii, it) in c.items.iter().enumerate() {
                validate_item(&format!("{cb}/items/{ii}"), &c.code, it, &mut errors);
                unique(format!("{cb}/items/{ii}/item_id"), &it.item_id, &mut errors);
                for (k, cr) in it.internal_evaluation_criteria.iter().enumerate() {
                    unique(
                        format!("{cb}/items/{ii}/internal_evaluation_criteria/{k}/criterion_id"),
                        &cr.criterion_id,
                        &mut errors,
                    );
                }
            }
        }
    }
    nonempty(
        "/cross_cutting_postures",
        &p.cross_cutting_postures,
        &mut errors,
    );
    for (i, x) in p.cross_cutting_postures.iter().enumerate() {
        let b = format!("/cross_cutting_postures/{i}");
        unique(format!("{b}/id"), &x.id, &mut errors);
        check_text(&format!("{b}/name"), &x.name, &mut errors);
        check_text(&format!("{b}/posture"), &x.posture, &mut errors);
    }
    nonempty("/module_laws", &p.module_laws, &mut errors);
    for (i, l) in p.module_laws.iter().enumerate() {
        let b = format!("/module_laws/{i}");
        unique(format!("{b}/id"), &l.id, &mut errors);
        check_text(&format!("{b}/law"), &l.law, &mut errors);
    }
    for role in Role::ALL {
        let n = p.role_focus.iter().filter(|r| r.role == role).count();
        if n != 1 {
            errors.push(PackError {
                path: "/role_focus".to_owned(),
                message: format!(
                    "role '{}' must have exactly one cut (has {n})",
                    role.as_str()
                ),
            });
        }
    }
    for (i, r) in p.role_focus.iter().enumerate() {
        let b = format!("/role_focus/{i}");
        check_text(&format!("{b}/identity"), &r.identity, &mut errors);
        nonempty(&format!("{b}/super_domains"), &r.super_domains, &mut errors);
        nonempty(&format!("{b}/worldviews"), &r.worldviews, &mut errors);
        for (k, code) in r.super_domains.iter().enumerate() {
            if !p.super_domains.iter().any(|s| &s.code == code) {
                errors.push(PackError {
                    path: format!("{b}/super_domains/{k}"),
                    message: format!("no super-domain '{code}'"),
                });
            }
        }
        for (k, id) in r.worldviews.iter().enumerate() {
            if !p.worldviews.iter().any(|w| &w.id == id) {
                errors.push(PackError {
                    path: format!("{b}/worldviews/{k}"),
                    message: format!("no worldview '{id}'"),
                });
            }
        }
    }
    errors
}

fn validate_item(b: &str, category_code: &str, it: &Item, errors: &mut Vec<PackError>) {
    if !is_code(&it.item_code, 2) {
        errors.push(PackError {
            path: format!("{b}/item_code"),
            message: "item code must be two digits".to_owned(),
        });
    }
    if it.item_id != format!("{category_code}{}", it.item_code) {
        errors.push(PackError {
            path: format!("{b}/item_id"),
            message: "item_id must be the category code followed by the item code".to_owned(),
        });
    }
    check_text(&format!("{b}/item_name"), &it.item_name, errors);
    check_text(
        &format!("{b}/detailed_description"),
        &it.detailed_description,
        errors,
    );
    let lists: [(&str, &[String]); 3] = [
        ("synthesis_instructions", &it.synthesis_instructions),
        ("applicability_signals", &it.applicability_signals),
        ("anti_signals", &it.anti_signals),
    ];
    for (name, list) in lists {
        nonempty(&format!("{b}/{name}"), list, errors);
        for (k, s) in list.iter().enumerate() {
            check_text(&format!("{b}/{name}/{k}"), s, errors);
        }
    }
    nonempty(
        &format!("{b}/internal_evaluation_criteria"),
        &it.internal_evaluation_criteria,
        errors,
    );
    for (k, c) in it.internal_evaluation_criteria.iter().enumerate() {
        let cb = format!("{b}/internal_evaluation_criteria/{k}");
        if !c.criterion_id.starts_with(&format!("{}-C", it.item_id)) {
            errors.push(PackError {
                path: format!("{cb}/criterion_id"),
                message: "criterion_id must be '<item_id>-C<n>'".to_owned(),
            });
        }
        check_text(&format!("{cb}/question"), &c.question, errors);
        check_text(&format!("{cb}/pass_condition"), &c.pass_condition, errors);
        check_text(&format!("{cb}/checklist"), &c.checklist, errors);
    }
    nonempty(
        &format!("{b}/signature_failures"),
        &it.signature_failures,
        errors,
    );
    for (k, f) in it.signature_failures.iter().enumerate() {
        let fb = format!("{b}/signature_failures/{k}");
        check_text(&format!("{fb}/failure"), &f.failure, errors);
        check_text(&format!("{fb}/guard"), &f.guard, errors);
    }
}

/// A loaded, validated pack with its file bytes and digest.
#[derive(Clone, Debug)]
pub struct LoadedPack {
    /// The typed pack.
    pub pack: Pack,
    /// `sha256:<hex>` of the file bytes.
    pub digest: String,
}

/// Read, type-check, validate and canonical-check a pack file's text.
/// Every problem is returned, named by JSON pointer.
pub fn load(text: &str) -> Result<LoadedPack, Vec<PackError>> {
    let value = json::parse(text).map_err(|e| {
        vec![PackError {
            path: String::new(),
            message: e.to_string(),
        }]
    })?;
    let pack = Pack::from_value(&value).map_err(|e| vec![e])?;
    let mut errors = validate(&pack);
    let canonical = pack.to_canonical();
    if canonical != text {
        let line = canonical
            .lines()
            .zip(text.lines())
            .position(|(a, b)| a != b)
            .unwrap_or_else(|| canonical.lines().count().min(text.lines().count()));
        errors.push(PackError {
            path: String::new(),
            message: format!(
                "the file is not in canonical layout (first difference at line {})",
                line + 1
            ),
        });
    }
    if errors.is_empty() {
        Ok(LoadedPack {
            pack,
            digest: pack_digest(text.as_bytes()),
        })
    } else {
        Err(errors)
    }
}
