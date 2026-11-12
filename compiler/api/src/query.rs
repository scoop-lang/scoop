use crate::schema::*;
use serde::Serialize;
use std::collections::BTreeMap;
use std::path::Path;

#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord, Serialize)]
pub struct Import {
    pub target: String,
    pub path: String,
    pub package: Vec<String>,
    pub name: String,
    pub coordinate: String,
    pub reexport: bool,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Entry {
    pub key: String,
    pub coordinate: Option<String>,
    pub declaration: Option<Declaration>,
    pub declaring_type: Option<DeclaringType>,
    pub imports: Vec<Import>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DeclaringType {
    pub key: String,
    pub qualified_name: String,
}

pub struct Catalog {
    pub target: String,
    entries: Vec<Entry>,
}

pub struct Selection<'a> {
    pub text: &'a str,
    pub exact: bool,
    pub cone: Option<&'a str>,
    pub receiver: Option<&'a str>,
    pub offset: usize,
    pub limit: usize,
}

#[derive(Serialize)]
pub struct ResultPage<'a> {
    pub schema: u32,
    pub kind: &'static str,
    pub status: &'static str,
    pub target: &'a str,
    pub total: usize,
    pub offset: usize,
    pub items: Vec<&'a Entry>,
}

fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("cannot read {}: {e}", path.display()))?;
    serde_json::from_slice(&bytes).map_err(|e| format!("invalid API data {}: {e}", path.display()))
}

fn schema(actual: u32) -> Result<(), String> {
    if actual == SCHEMA {
        Ok(())
    } else {
        Err(format!(
            "unsupported API schema {actual}; expected {SCHEMA}"
        ))
    }
}

impl Catalog {
    pub fn read(root: &Path, target: Option<&str>) -> Result<Self, String> {
        let index: Index = read(&root.join("api/index.json")).map_err(|error|
            format!("{error}; generate documentation explicitly with scoop doc, or update installed documentation with scoop-install docs"))?;
        schema(index.schema)?;
        if let Some(target) = target
            && index.target != target
        {
            return Err(format!(
                "documentation target is {}, requested {target}",
                index.target
            ));
        }
        let mut entries = BTreeMap::<String, Entry>::new();
        let mut alternates = BTreeMap::new();
        let mut bindings = Vec::new();
        for reference in index.cones {
            let cone: Cone = read(&root.join(&reference.file))?;
            schema(cone.schema)?;
            if reference.coordinate != cone.coordinate {
                return Err(format!("API coordinate mismatch in {}", reference.file));
            }
            for declaration in cone.declarations {
                for alternate in &declaration.alternate_keys {
                    alternates.insert(alternate.clone(), declaration.key.clone());
                }
                let key = declaration.key.clone();
                let entry = Entry {
                    key: key.clone(),
                    coordinate: Some(cone.coordinate.clone()),
                    declaration: Some(declaration),
                    declaring_type: None,
                    imports: Vec::new(),
                };
                if entries.insert(key.clone(), entry).is_some() {
                    return Err(format!("duplicate API declaration {key}"));
                }
            }
            bindings.extend(
                cone.bindings
                    .into_iter()
                    .map(|binding| (cone.coordinate.clone(), binding)),
            );
        }
        let mut imports = BTreeMap::<String, Vec<Import>>::new();
        for (coordinate, binding) in bindings {
            let key = alternates
                .get(&binding.target)
                .unwrap_or(&binding.target)
                .clone();
            imports.entry(key.clone()).or_default().push(Import {
                target: key.clone(),
                path: binding.path(),
                package: binding.package,
                name: binding.name,
                coordinate,
                reexport: binding.reexport,
            });
            entries.entry(key.clone()).or_insert(Entry {
                key,
                coordinate: None,
                declaration: None,
                declaring_type: None,
                imports: Vec::new(),
            });
        }
        // Parent edges describe the importable container; no language lookup occurs here.
        let mut resolved = Vec::with_capacity(entries.len());
        for mut entry in entries.values().cloned() {
            entry.declaring_type = entry
                .declaration
                .as_ref()
                .filter(|d| matches!(d.kind.as_str(), "function" | "property" | "field"))
                .and_then(|d| d.parent.as_ref())
                .and_then(|key| entries.get(key))
                .and_then(|owner| owner.declaration.as_ref())
                .filter(|d| {
                    matches!(
                        d.kind.as_str(),
                        "class" | "struct" | "enum" | "interface" | "object"
                    )
                })
                .map(|owner| DeclaringType {
                    key: owner.key.clone(),
                    qualified_name: owner.qualified_name.clone(),
                });
            let mut current = Some(entry.key.as_str());
            let mut seen = std::collections::BTreeSet::new();
            while let Some(key) = current {
                if !seen.insert(key) {
                    return Err(format!("cyclic API parent at {key}"));
                }
                if let Some(bindings) = imports.get(key) {
                    entry.imports.extend(bindings.iter().cloned());
                }
                current = entries
                    .get(key)
                    .and_then(|e| e.declaration.as_ref())
                    .and_then(|d| d.parent.as_deref());
            }
            entry.imports.sort();
            entry.imports.dedup();
            resolved.push(entry);
        }
        Ok(Self {
            target: index.target,
            entries: resolved,
        })
    }

    pub fn query(&self, selection: Selection<'_>) -> ResultPage<'_> {
        let words: Vec<_> = selection
            .text
            .split_whitespace()
            .map(str::to_lowercase)
            .collect();
        let exact = selection.text.to_lowercase();
        let mut matches = self
            .entries
            .iter()
            .filter(|entry| {
                if let Some(cone) = selection.cone {
                    let coordinate_matches = |coordinate: &str| {
                        coordinate == cone || coordinate.split(':').nth(1) == Some(cone)
                    };
                    if !entry.coordinate.as_deref().is_some_and(coordinate_matches)
                        && !entry
                            .imports
                            .iter()
                            .any(|import| coordinate_matches(&import.coordinate))
                    {
                        return false;
                    }
                }
                if let Some(receiver) = selection.receiver {
                    let receiver = receiver.to_lowercase();
                    let explicit = entry
                        .declaration
                        .as_ref()
                        .and_then(|d| d.receiver.as_ref())
                        .is_some_and(|text| text.to_lowercase().contains(&receiver));
                    let member = entry.declaring_type.as_ref().is_some_and(|owner| {
                        owner.qualified_name.to_lowercase().contains(&receiver)
                    });
                    if !explicit && !member {
                        return false;
                    }
                }
                if selection.exact {
                    entry
                        .names()
                        .iter()
                        .any(|name| name.to_lowercase() == exact)
                } else {
                    let text = entry.search_text().to_lowercase();
                    words.iter().all(|word| text.contains(word))
                }
            })
            .collect::<Vec<_>>();
        matches.sort_by_key(|entry| {
            (
                !entry
                    .names()
                    .iter()
                    .any(|name| name.to_lowercase() == exact),
                entry.coordinate.as_deref().unwrap_or(""),
                entry
                    .declaration
                    .as_ref()
                    .map(|d| d.qualified_name.as_str())
                    .unwrap_or(""),
                entry.key.as_str(),
            )
        });
        let total = matches.len();
        let items = matches
            .into_iter()
            .skip(selection.offset)
            .take(selection.limit)
            .collect();
        ResultPage {
            schema: SCHEMA,
            kind: if selection.exact { "show" } else { "search" },
            status: if total == 0 { "empty" } else { "ok" },
            target: &self.target,
            total,
            offset: selection.offset,
            items,
        }
    }
}

impl Entry {
    fn names(&self) -> Vec<String> {
        let mut names = vec![self.key.clone()];
        if let Some(declaration) = &self.declaration {
            names.extend([declaration.name.clone(), declaration.qualified_name.clone()]);
        }
        for import in &self.imports {
            if import.target == self.key {
                names.extend([import.path.clone(), import.name.clone()]);
                names.push(
                    import
                        .package
                        .iter()
                        .chain(std::iter::once(&import.name))
                        .cloned()
                        .collect::<Vec<_>>()
                        .join("."),
                );
            }
        }
        names
    }

    fn search_text(&self) -> String {
        let mut text = self.names().join(" ");
        if let Some(coordinate) = &self.coordinate {
            text.push(' ');
            text.push_str(coordinate);
        }
        for import in &self.imports {
            text.push_str(&format!(" {}", import.coordinate));
        }
        if let Some(declaration) = &self.declaration {
            text.push_str(&format!(
                " {} {} {}",
                declaration.kind,
                declaration.receiver.as_deref().unwrap_or(""),
                declaration.aliases.join(" ")
            ));
            if let Some(documentation) = &declaration.documentation {
                text.push(' ');
                text.push_str(&documentation.markdown);
                for tag in &documentation.tags {
                    text.push_str(&format!(" {} {}", tag.label, tag.markdown));
                }
            }
        }
        text
    }
}
