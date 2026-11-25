//! An item's page on docs.rs, for a crate published on crates.io.

use super::{Part, crate_of, own_name};
use vz_core::code_index::{Entry, Kind};

/// The anchor on its type's page that a member of each kind has
fn anchor(kind: Kind) -> Option<&'static str> {
    match kind {
        Kind::Fn => Some("method"),
        Kind::Field => Some("structfield"),
        Kind::Variant => Some("variant"),
        Kind::Const => Some("associatedconstant"),
        Kind::Type => Some("associatedtype"),
        _ => None,
    }
}

/// Whether items of `kind` have members on their page
fn has_members(kind: Kind) -> bool {
    matches!(
        kind,
        Kind::Struct | Kind::Enum | Kind::Union | Kind::Trait | Kind::Type
    )
}

/// `mods` as the directories of a docs.rs address
fn dirs(mods: &[&str]) -> String {
    mods.iter().flat_map(|m| [*m, "/"]).collect()
}

impl Part {
    /// The item's page on docs.rs, or `None` for a crate that is not published or the index knows nothing of
    pub(crate) fn docs_rs_url(&self, item: &Entry) -> Option<String> {
        let path: Vec<&str> = item.path.split("::").collect();
        let info = self.index.crates.get(path[0]).filter(|c| c.published)?;
        let root = format!("https://docs.rs/{}/latest/{}/", info.package, path[0]);
        let (kind, name) = (item.kind, path[path.len() - 1]);
        if kind == Kind::Macro {
            return Some(format!("{root}macro.{name}.html"));
        }
        if path.len() > 2
            && let Some(anchor) = anchor(kind)
        {
            // a member sits on its type's page; its path may name the module of the `impl`, not the type's own
            let parent = path[..path.len() - 1].join("::");
            let types: Vec<&Entry> = self
                .index
                .items
                .iter()
                .filter(|i| {
                    has_members(i.kind) && crate_of(&i.path) == path[0] && i.reexports.is_none()
                })
                .collect();
            let owner = types.iter().find(|i| i.path == parent).or_else(|| {
                types
                    .iter()
                    .find(|i| own_name(&i.path) == path[path.len() - 2])
            });
            match owner {
                Some(owner) => {
                    let segs: Vec<&str> = owner.path.split("::").collect();
                    let page = format!("{}.{}.html", owner.kind.name(), segs[segs.len() - 1]);
                    return Some(format!(
                        "{root}{}{page}#{anchor}.{name}",
                        dirs(&segs[1..segs.len() - 1])
                    ));
                }
                None if matches!(kind, Kind::Field | Kind::Variant) => return None,
                None => {}
            }
        }
        Some(format!(
            "{root}{}{}.{name}.html",
            dirs(&path[1..path.len() - 1]),
            kind.name()
        ))
    }
}
