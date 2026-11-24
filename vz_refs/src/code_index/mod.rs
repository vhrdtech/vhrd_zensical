//! The index of public items a book's code links name.
//!
//! Every public item of the book's library crates, found by walking each library's module tree from its root as
//! the compiler does (`mod x;` to `x.rs` or `x/mod.rs`, `#[path]` honoured, `#[cfg(test)]` modules left out). Each
//! entry has its path through the modules it is defined in (`ww_tape::writer::resume::TapeWriter::resume_on`), its
//! kind, its `#[cfg]` when it has one, file and lines, its signature (its whole source when short), its docs, and the
//! address of its lines on the forge at one commit. A `pub use` adds the items it re-exports a second time, under
//! the path it gives them, with the same file and lines.
//!
//! The index is written at every book build and never kept in git, so it always matches the code: a link to a
//! renamed or removed item fails the build in the change that renames it.

mod files;

pub use files::FileEntry;

use serde::{Deserialize, Serialize};
use sl_model::{Package, Project, SourceFile, TargetKind};
use syn::spanned::Spanned;
use syn::{Attribute, Fields, ImplItem, Item, Visibility};

/// The version of the index's layout; the Markdown extension refuses an index of another
pub(crate) const FORMAT: u32 = 2;

/// Lines of an item's source shown whole at most; a longer item shows its signature alone
const WHOLE_LINES_MAX: usize = 14;

/// What an item is
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Kind {
    /// A function or method
    Fn,
    /// A struct
    Struct,
    /// A union
    Union,
    /// A named field of a struct or a union
    Field,
    /// An enum
    Enum,
    /// A variant of an enum
    Variant,
    /// A trait
    Trait,
    /// A type alias, associated types of a trait included
    Type,
    /// A constant, associated ones included
    Const,
    /// A static
    Static,
    /// An exported `macro_rules!` macro
    Macro,
}

/// One public item
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Entry {
    /// Its path from its crate's name through the modules it is defined in
    pub path: String,
    /// What it is
    pub kind: Kind,
    /// Its `#[cfg(...)]` predicate as written, several joined in `all(...)`; two arms of one item differ in it
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cfg: Option<String>,
    /// The file it is defined in, relative to the project root
    pub file: String,
    /// Its first and last line in the file, from 1: from its visibility to its end, docs and attributes left out
    pub lines: [usize; 2],
    /// Its signature, or its whole source when short, dedented
    pub source: String,
    /// Its docs, as written: Markdown with rustdoc's intra-doc links
    pub doc: String,
    /// Its lines on the forge at the index's commit
    pub url: String,
    /// For a re-export, the path the item is defined at
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reexports: Option<String>,
}

/// A library crate of the index, as docs.rs addresses it
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct CrateEntry {
    /// The package name, which docs.rs puts first in its addresses (`ww-tape`; the library's own name is `ww_tape`)
    pub package: String,
    /// Whether the package is published on crates.io, as the project's `[book] on-crates-io` declares
    pub published: bool,
}

/// The index file
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Index {
    /// The version of its layout, 2; the Markdown extension refuses an index of another
    pub format: u32,
    /// The commit the addresses point at
    pub commit: String,
    /// The address a file's path follows to open it at the commit, `#L1-L9` after it for lines
    pub blob: String,
    /// The indexed libraries by library name, the first segment of their items' paths
    #[serde(default)]
    pub crates: std::collections::BTreeMap<String, CrateEntry>,
    /// The public items, sorted by path
    pub items: Vec<Entry>,
    /// The project's text files, which file references name, sorted by path
    pub files: Vec<FileEntry>,
}

/// Where the entries' addresses point
#[derive(Clone, Copy, Debug)]
pub struct Forge<'a> {
    /// The repository's address (`https://github.com/vhrdtech/ww_tape`)
    pub repository: &'a str,
    /// The full commit hash the book is built from
    pub commit: &'a str,
}

impl Forge<'_> {
    /// The address a file's path follows, as GitHub, Gitea and Forgejo address files at a commit
    fn blob(&self) -> String {
        format!(
            "{}/blob/{}/",
            self.repository.trim_end_matches('/'),
            self.commit
        )
    }

    /// A file's lines
    fn url(&self, file: &str, [from, to]: [usize; 2]) -> String {
        format!("{}{file}#L{from}-L{to}", self.blob())
    }
}

/// Why an index cannot be built
#[derive(Debug, PartialEq, Eq)]
pub enum IndexError {
    /// A crate the settings name is no library of the workspace
    UnknownCrate {
        /// The name as written
        name: String,
        /// The libraries there are
        known: Vec<String>,
    },
    /// A source file does not parse (`file:line:column: message`)
    Parse(String),
    /// A `mod x;` whose file is not there
    MissingModule {
        /// The declaring file
        file: String,
        /// The module's name
        name: String,
    },
}

impl std::fmt::Display for IndexError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnknownCrate { name, known } => write!(
                f,
                "`{name}` is no library of the workspace; the libraries are {}",
                known.join(", ")
            ),
            Self::Parse(e) => write!(f, "{e}"),
            Self::MissingModule { file, name } => {
                write!(f, "{file}: `mod {name};` has no file")
            }
        }
    }
}

impl std::error::Error for IndexError {}

/// The index of `crates` (library target names; empty: every library of the workspace) and of the text files git
/// tracks, leaving out the directories `skip` (relative to the root, such as the built site)
///
/// # Errors
///
/// A crate is no library of the workspace, a file does not parse, or a declared module has no file.
pub fn index(
    project: &Project,
    crates: &[String],
    skip: &[&str],
    on_crates_io: &[String],
    forge: Forge<'_>,
) -> Result<Index, IndexError> {
    let libs: Vec<(&Package, &str, &str)> = project
        .packages
        .iter()
        .flat_map(|p| {
            p.targets
                .iter()
                .zip(&p.target_roots)
                .filter(|((k, _), _)| matches!(k, TargetKind::Lib | TargetKind::ProcMacro))
                .map(move |((_, name), root)| (p, name.as_str(), root.as_str()))
        })
        .collect();
    for c in crates {
        if !libs.iter().any(|(_, name, _)| name == c) {
            let mut known: Vec<String> = libs.iter().map(|(_, n, _)| (*n).to_owned()).collect();
            known.sort();
            return Err(IndexError::UnknownCrate {
                name: c.clone(),
                known,
            });
        }
    }
    let mut items = Vec::new();
    let mut uses = Vec::new();
    let mut indexed = std::collections::BTreeMap::new();
    for (pkg, name, root) in libs {
        if !crates.is_empty() && !crates.iter().any(|c| c == name) {
            continue;
        }
        indexed.insert(
            name.to_owned(),
            CrateEntry {
                package: pkg.name.clone(),
                published: on_crates_io.contains(&pkg.name),
            },
        );
        let files = project.sources(pkg);
        let mut walk = Walk {
            files,
            forge,
            out: &mut items,
            uses: &mut uses,
        };
        walk.file(root, &[name.to_owned()])?;
    }
    reexport(&mut items, &uses);
    items.sort_by(|a, b| a.path.cmp(&b.path).then(a.lines.cmp(&b.lines)));
    Ok(Index {
        format: FORMAT,
        commit: forge.commit.to_owned(),
        blob: forge.blob(),
        crates: indexed,
        items,
        files: files::files(project, skip),
    })
}

/// The docs in `attrs`, as written: Markdown, dedented, without leading and trailing blank lines
fn doc(attrs: &[Attribute]) -> String {
    let mut lines: Vec<String> = Vec::new();
    for a in attrs.iter().filter(|a| a.path().is_ident("doc")) {
        let syn::Meta::NameValue(nv) = &a.meta else {
            continue;
        };
        let syn::Expr::Lit(syn::ExprLit {
            lit: syn::Lit::Str(s),
            ..
        }) = &nv.value
        else {
            continue;
        };
        let value = s.value();
        // an empty `///` line is an empty value, which has no lines: a blank line all the same
        if value.trim().is_empty() {
            lines.push(String::new());
        } else {
            lines.extend(value.lines().map(|l| l.trim_end().to_owned()));
        }
    }
    let indent = lines
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    let lines: Vec<&str> = lines
        .iter()
        .map(|l| l.get(indent..).unwrap_or(""))
        .collect();
    lines.join("\n").trim_matches('\n').to_owned()
}

fn is_pub(v: &Visibility) -> bool {
    matches!(v, Visibility::Public(_))
}

/// Whether attributes hold `#[cfg(test)]`
fn is_cfg_test(attrs: &[Attribute]) -> bool {
    attrs.iter().any(|a| {
        a.path().is_ident("cfg") && a.parse_args::<syn::Ident>().is_ok_and(|i| i == "test")
    })
}

/// The `#[cfg(...)]` predicates in `attrs` as written, several joined in `all(...)`
fn cfg(attrs: &[Attribute]) -> Option<String> {
    let preds: Vec<String> = attrs
        .iter()
        .filter(|a| a.path().is_ident("cfg"))
        .filter_map(|a| match &a.meta {
            syn::Meta::List(l) => Some(predicate(&l.tokens)),
            _ => None,
        })
        .collect();
    match preds.len() {
        0 => None,
        1 => preds.into_iter().next(),
        _ => Some(format!("all({})", preds.join(", "))),
    }
}

/// A cfg predicate's tokens as a reader writes them: `not(unix)`, `feature = "std"`, `any(a, b)`
fn predicate(tokens: &proc_macro2::TokenStream) -> String {
    let mut out = String::new();
    for t in tokens.clone() {
        match t {
            proc_macro2::TokenTree::Group(g) => {
                out.push('(');
                out.push_str(&predicate(&g.stream()));
                out.push(')');
            }
            proc_macro2::TokenTree::Punct(p) if p.as_char() == ',' => out.push_str(", "),
            proc_macro2::TokenTree::Punct(p) if p.as_char() == '=' => out.push_str(" = "),
            other => out.push_str(&other.to_string()),
        }
    }
    out
}

/// The names a `pub use` tree in `module` gives, `prefix` being the path written before this part of the tree
fn uses(module: &[String], prefix: &mut Vec<String>, tree: &syn::UseTree, out: &mut Vec<Use>) {
    match tree {
        syn::UseTree::Path(p) => {
            prefix.push(p.ident.to_string());
            uses(module, prefix, &p.tree, out);
            prefix.pop();
        }
        syn::UseTree::Name(n) => out.push(Use {
            module: module.to_vec(),
            target: [prefix.as_slice(), &[n.ident.to_string()]].concat(),
            name: Some(n.ident.to_string()),
        }),
        syn::UseTree::Rename(r) => out.push(Use {
            module: module.to_vec(),
            target: [prefix.as_slice(), &[r.ident.to_string()]].concat(),
            name: Some(r.rename.to_string()),
        }),
        syn::UseTree::Glob(_) => out.push(Use {
            module: module.to_vec(),
            target: prefix.clone(),
            name: None,
        }),
        syn::UseTree::Group(g) => {
            for t in &g.items {
                uses(module, prefix, t, out);
            }
        }
    }
}

/// The absolute paths a use path written in `module` can mean: from `crate`, `self` or `super`, else relative to
/// the module and then from a crate's name
fn absolute(module: &[String], target: &[String]) -> Vec<String> {
    let mut base: Vec<String> = module.to_vec();
    let mut rest = target;
    match rest.first().map(String::as_str) {
        Some("crate") => {
            base.truncate(1);
            rest = &rest[1..];
        }
        Some("self" | "super") => {
            while let Some(first) = rest.first() {
                match first.as_str() {
                    "self" => {}
                    "super" => {
                        base.pop();
                    }
                    _ => break,
                }
                rest = &rest[1..];
            }
        }
        _ => {
            let relative = [base.as_slice(), rest].concat().join("::");
            return vec![relative, rest.join("::")];
        }
    }
    vec![[base.as_slice(), rest].concat().join("::")]
}

/// Adds the items each `pub use` re-exports under the path it gives them, re-exports of re-exports included
fn reexport(items: &mut Vec<Entry>, uses: &[Use]) {
    let mut known: std::collections::BTreeSet<(String, String, [usize; 2])> = items
        .iter()
        .map(|e| (e.path.clone(), e.file.clone(), e.lines))
        .collect();
    // each round follows one more step of a chain of re-exports; a chain is short, a cycle adds nothing
    for _ in 0..8 {
        let mut added = Vec::new();
        for u in uses {
            let under = |e: &Entry, target: &str| -> Option<String> {
                let rest = e.path.strip_prefix(target)?;
                match &u.name {
                    Some(name) if rest.is_empty() || rest.starts_with("::") => {
                        Some(format!("{}::{name}{rest}", u.module.join("::")))
                    }
                    None if rest.starts_with("::") => {
                        Some(format!("{}{rest}", u.module.join("::")))
                    }
                    _ => None,
                }
            };
            let found = absolute(&u.module, &u.target).into_iter().find_map(|t| {
                let found: Vec<(String, &Entry)> = items
                    .iter()
                    .filter_map(|e| Some((under(e, &t)?, e)))
                    .collect();
                (!found.is_empty()).then_some(found)
            });
            for (path, e) in found.unwrap_or_default() {
                if known.insert((path.clone(), e.file.clone(), e.lines)) {
                    added.push(Entry {
                        path,
                        reexports: Some(e.reexports.clone().unwrap_or_else(|| e.path.clone())),
                        ..e.clone()
                    });
                }
            }
        }
        if added.is_empty() {
            break;
        }
        items.extend(added);
    }
}

/// `#[path = "..."]` on a module
fn path_attr(attrs: &[Attribute]) -> Option<String> {
    attrs.iter().find_map(|a| {
        if !a.path().is_ident("path") {
            return None;
        }
        match &a.meta {
            syn::Meta::NameValue(syn::MetaNameValue {
                value:
                    syn::Expr::Lit(syn::ExprLit {
                        lit: syn::Lit::Str(s),
                        ..
                    }),
                ..
            }) => Some(s.value()),
            _ => None,
        }
    })
}

/// The directory of a path inside a package, with a trailing slash unless empty
fn dir_of(path: &str) -> &str {
    path.rfind('/').map_or("", |i| &path[..=i])
}

struct Walk<'a> {
    files: &'a [SourceFile],
    forge: Forge<'a>,
    out: &'a mut Vec<Entry>,
    uses: &'a mut Vec<Use>,
}

/// One name a `pub use` gives: in `module`, `name` for the item at `target` (relative as written), or every item
/// under `target` for a glob
#[derive(Debug)]
struct Use {
    module: Vec<String>,
    target: Vec<String>,
    name: Option<String>,
}

/// One file being walked: its lines and where its child modules' files are
struct At<'a> {
    file: &'a SourceFile,
    lines: Vec<&'a str>,
    /// The directory a `mod x;` in this file looks in, inside the package, with a trailing slash unless empty
    children: String,
}

impl Walk<'_> {
    /// Walks the module in `in_package` (a path inside the package) whose path is `module`
    fn file(&mut self, in_package: &str, module: &[String]) -> Result<(), IndexError> {
        let Some(file) = self.files.iter().find(|f| f.in_package == in_package) else {
            return Ok(());
        };
        let ast = file.ast().map_err(|e| IndexError::Parse(e.to_owned()))?;
        let stem = in_package.rsplit('/').next().unwrap_or(in_package);
        let children = if matches!(stem, "lib.rs" | "main.rs" | "mod.rs") {
            dir_of(in_package).to_owned()
        } else {
            format!("{}/", in_package.trim_end_matches(".rs"))
        };
        let at = At {
            file,
            lines: file.text.lines().collect(),
            children,
        };
        self.items(&at, module, &at.children.clone(), &ast.items)
    }

    /// Walks `items` of `module`, whose out-of-line children are in `children`
    fn items(
        &mut self,
        at: &At<'_>,
        module: &[String],
        children: &str,
        items: &[Item],
    ) -> Result<(), IndexError> {
        let join = |name: &syn::Ident| format!("{}::{name}", module.join("::"));
        for item in items {
            match item {
                Item::Struct(s) if is_pub(&s.vis) => self.structure(at, join(&s.ident), s),
                Item::Union(u) if is_pub(&u.vis) => self.union(at, join(&u.ident), u),
                Item::Enum(e) if is_pub(&e.vis) => self.enumeration(at, join(&e.ident), e),
                Item::Trait(t) if is_pub(&t.vis) => self.tr(at, join(&t.ident), t),
                Item::Macro(m) => self.exported_macro(at, module, m),
                Item::Mod(m) if !is_cfg_test(&m.attrs) => self.module(at, module, children, m)?,
                Item::Use(u) if is_pub(&u.vis) => uses(module, &mut Vec::new(), &u.tree, self.uses),
                Item::Impl(i) if i.trait_.is_none() => {
                    if let syn::Type::Path(p) = &*i.self_ty
                        && let Some(ty) = p.path.segments.last()
                    {
                        self.inherent(at, &join(&ty.ident), &i.items);
                    }
                }
                _ => self.single(at, module, item),
            }
        }
        Ok(())
    }

    /// A public function, type alias, constant or static: one entry without parts
    fn single(&mut self, at: &At<'_>, module: &[String], item: &Item) {
        let (ident, kind, attrs, open) = match item {
            Item::Fn(f) if is_pub(&f.vis) => (
                &f.sig.ident,
                Kind::Fn,
                &f.attrs,
                Some(f.block.brace_token.span.open().start().line),
            ),
            Item::Type(t) if is_pub(&t.vis) => (&t.ident, Kind::Type, &t.attrs, None),
            Item::Const(c) if is_pub(&c.vis) => (&c.ident, Kind::Const, &c.attrs, None),
            Item::Static(s) if is_pub(&s.vis) => (&s.ident, Kind::Static, &s.attrs, None),
            _ => return,
        };
        let path = format!("{}::{ident}", module.join("::"));
        self.push(at, path, kind, attrs, item, open);
    }

    /// A `macro_rules!` macro, when exported: it lives at the crate's root, wherever it is written
    fn exported_macro(&mut self, at: &At<'_>, module: &[String], m: &syn::ItemMacro) {
        if !m.attrs.iter().any(|a| a.path().is_ident("macro_export")) {
            return;
        }
        if let Some(ident) = &m.ident {
            let krate = module.first().map_or("", String::as_str);
            let path = format!("{krate}::{ident}");
            self.push(at, path, Kind::Macro, &m.attrs, m, None);
        }
    }

    /// A public struct and its public named fields
    fn structure(&mut self, at: &At<'_>, name: String, s: &syn::ItemStruct) {
        let open = match &s.fields {
            Fields::Named(n) => Some(n.brace_token.span.open().start().line),
            _ => None,
        };
        if let Fields::Named(n) = &s.fields {
            for f in n.named.iter().filter(|f| is_pub(&f.vis)) {
                if let Some(ident) = &f.ident {
                    self.push(
                        at,
                        format!("{name}::{ident}"),
                        Kind::Field,
                        &f.attrs,
                        f,
                        None,
                    );
                }
            }
        }
        self.push(at, name, Kind::Struct, &s.attrs, s, open);
    }

    /// A public union and its public fields
    fn union(&mut self, at: &At<'_>, name: String, u: &syn::ItemUnion) {
        for f in u.fields.named.iter().filter(|f| is_pub(&f.vis)) {
            if let Some(ident) = &f.ident {
                self.push(
                    at,
                    format!("{name}::{ident}"),
                    Kind::Field,
                    &f.attrs,
                    f,
                    None,
                );
            }
        }
        let open = u.fields.brace_token.span.open().start().line;
        self.push(at, name, Kind::Union, &u.attrs, u, Some(open));
    }

    /// A public trait and its functions, constants and types, all public with it
    fn tr(&mut self, at: &At<'_>, name: String, t: &syn::ItemTrait) {
        for it in &t.items {
            match it {
                syn::TraitItem::Fn(f) => {
                    let open = f
                        .default
                        .as_ref()
                        .map(|b| b.brace_token.span.open().start().line);
                    let path = format!("{name}::{}", f.sig.ident);
                    self.push(at, path, Kind::Fn, &f.attrs, it, open);
                }
                syn::TraitItem::Const(c) => {
                    let path = format!("{name}::{}", c.ident);
                    self.push(at, path, Kind::Const, &c.attrs, it, None);
                }
                syn::TraitItem::Type(ty) => {
                    let path = format!("{name}::{}", ty.ident);
                    self.push(at, path, Kind::Type, &ty.attrs, it, None);
                }
                _ => {}
            }
        }
        let open = t.brace_token.span.open().start().line;
        self.push(at, name, Kind::Trait, &t.attrs, t, Some(open));
    }

    /// A public enum and its variants
    fn enumeration(&mut self, at: &At<'_>, name: String, e: &syn::ItemEnum) {
        for v in &e.variants {
            let path = format!("{name}::{}", v.ident);
            self.push(at, path, Kind::Variant, &v.attrs, v, None);
        }
        let open = e.brace_token.span.open().start().line;
        self.push(at, name, Kind::Enum, &e.attrs, e, Some(open));
    }

    /// The public functions and constants of an inherent impl of `ty`
    fn inherent(&mut self, at: &At<'_>, ty: &str, items: &[ImplItem]) {
        for it in items {
            match it {
                ImplItem::Fn(f) if is_pub(&f.vis) => {
                    let open = f.block.brace_token.span.open().start().line;
                    let path = format!("{ty}::{}", f.sig.ident);
                    self.push(at, path, Kind::Fn, &f.attrs, it, Some(open));
                }
                ImplItem::Const(c) if is_pub(&c.vis) => {
                    let path = format!("{ty}::{}", c.ident);
                    self.push(at, path, Kind::Const, &c.attrs, it, None);
                }
                _ => {}
            }
        }
    }

    /// A module, inline or in its own file
    fn module(
        &mut self,
        at: &At<'_>,
        module: &[String],
        children: &str,
        m: &syn::ItemMod,
    ) -> Result<(), IndexError> {
        let mut inner = module.to_vec();
        inner.push(m.ident.to_string());
        if let Some((_, nested)) = &m.content {
            let dir = format!("{children}{}/", m.ident);
            self.items(at, &inner, &dir, nested)
        } else {
            let file = self.module_file(at, children, &m.ident.to_string(), &m.attrs)?;
            self.file(&file, &inner)
        }
    }

    /// The file of `mod name;`: its `#[path]` beside the declaring file, else `name.rs` or `name/mod.rs`
    fn module_file(
        &self,
        at: &At<'_>,
        children: &str,
        name: &str,
        attrs: &[Attribute],
    ) -> Result<String, IndexError> {
        let candidates = match path_attr(attrs) {
            Some(p) => vec![format!("{}{p}", dir_of(&at.file.in_package))],
            None => vec![
                format!("{children}{name}.rs"),
                format!("{children}{name}/mod.rs"),
            ],
        };
        candidates
            .into_iter()
            .find(|c| self.files.iter().any(|f| &f.in_package == c))
            .ok_or_else(|| IndexError::MissingModule {
                file: at.file.rel.clone(),
                name: name.to_owned(),
            })
    }

    /// Adds an item spanning `node`, whose body opens on line `open` (none: no body to cut off)
    fn push(
        &mut self,
        at: &At<'_>,
        path: String,
        kind: Kind,
        attrs: &[Attribute],
        node: &dyn Spanned,
        open: Option<usize>,
    ) {
        let span = node.span();
        // the item's span starts at its first attribute; the reader wants it from the line after them, or from
        // the attribute's own line when the item goes on after it there (`#[inline] pub fn f() {}`)
        let start = attrs
            .iter()
            .map(|a| {
                let end = a.span().end();
                let rest = at.lines.get(end.line.wrapping_sub(1)).map_or("", |l| {
                    l.char_indices()
                        .nth(end.column)
                        .map_or("", |(i, _)| &l[i..])
                });
                if rest.trim().is_empty() {
                    end.line + 1
                } else {
                    end.line
                }
            })
            .max()
            .unwrap_or(span.start().line)
            .max(span.start().line);
        let end = span.end().line.max(start);
        let lines = [start, end];
        self.out.push(Entry {
            path,
            kind,
            cfg: cfg(attrs),
            file: at.file.rel.clone(),
            lines,
            source: source(&at.lines, start, end, open),
            doc: doc(attrs),
            url: self.forge.url(&at.file.rel, lines),
            reexports: None,
        });
    }
}

/// The lines `from..=to` (from 1), dedented
fn text(lines: &[&str], from: usize, to: usize) -> String {
    let picked = lines
        .get(from.saturating_sub(1)..to.min(lines.len()))
        .unwrap_or_default();
    let indent = picked
        .iter()
        .filter(|l| !l.trim().is_empty())
        .map(|l| l.len() - l.trim_start().len())
        .min()
        .unwrap_or(0);
    picked
        .iter()
        .map(|l| l.get(indent..).unwrap_or("").trim_end())
        .collect::<Vec<_>>()
        .join("\n")
}

/// An item's whole source when short or without a body, else the lines before its body, the brace cut off
fn source(lines: &[&str], start: usize, end: usize, open: Option<usize>) -> String {
    match open {
        Some(open) if end + 1 - start > WHOLE_LINES_MAX => text(lines, start, open)
            .trim_end()
            .trim_end_matches('{')
            .trim_end()
            .to_owned(),
        _ => text(lines, start, end),
    }
}

#[cfg(test)]
mod tests;
