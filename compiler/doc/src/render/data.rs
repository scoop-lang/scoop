//! The public API format is a projection, not the private ConeInput cache format.

use super::*;
use scoop_api as api;

impl SiteRenderer<'_, '_> {
    pub fn api_index(&self, target: &str) -> Result<RenderedFile> {
        let index = api::Index {
            schema: api::SCHEMA,
            target: target.to_owned(),
            cones: self
                .catalog
                .cones
                .values()
                .map(|cone| api::ConeReference {
                    key: cone.key.0.clone(),
                    coordinate: cone.name.clone(),
                    file: format!("api/{}.json", cone.key.0),
                })
                .collect(),
        };
        Ok(RenderedFile {
            path: "api/index.json".into(),
            contents: serde_json::to_vec(&index).map_err(RenderError::Encoding)?,
        })
    }

    pub(super) fn api_data(&self, cone: &ConeInput) -> Result<RenderedFile> {
        let declarations = self
            .catalog
            .items
            .values()
            .filter(|(owner, item)| {
                owner.key == cone.key
                    && !matches!(
                        item.kind,
                        ApiKind::Parameter
                            | ApiKind::TypeParameter
                            | ApiKind::ContextParameter
                            | ApiKind::Getter
                            | ApiKind::Setter
                    )
            })
            .map(|(_, item)| self.api_declaration(cone, item))
            .collect();
        let bindings = cone
            .bindings
            .iter()
            .filter(|binding| self.catalog.binding_visible(binding))
            .map(|binding| api::Binding {
                package: binding.package.clone(),
                name: binding.name.clone(),
                target: binding.target.0.clone(),
                reexport: binding.reexport,
            })
            .collect();
        let data = api::Cone {
            schema: api::SCHEMA,
            coordinate: cone.name.clone(),
            declarations,
            bindings,
        };
        Ok(RenderedFile {
            path: format!("api/{}.json", cone.key.0),
            contents: serde_json::to_vec(&data).map_err(RenderError::Encoding)?,
        })
    }

    fn api_declaration(&self, cone: &ConeInput, item: &ApiItem) -> api::Declaration {
        let mut owners = Vec::new();
        let mut parent = item.parent.as_ref();
        while let Some(key) = parent {
            let owner = self.catalog.items[key].1;
            owners.push(owner.name.clone());
            parent = owner.parent.as_ref();
        }
        owners.reverse();
        let qualified_name = item
            .package
            .iter()
            .cloned()
            .chain(owners)
            .chain(std::iter::once(item.name.clone()))
            .collect::<Vec<_>>()
            .join(".");
        let source = cone
            .sources
            .iter()
            .find(|source| source.key == item.location.source);
        api::Declaration {
            key: item.key.0.clone(),
            alternate_keys: item
                .alternate_keys
                .iter()
                .map(|key| key.0.clone())
                .collect(),
            parent: item.parent.as_ref().map(|key| key.0.clone()),
            package: item.package.clone(),
            name: item.name.clone(),
            qualified_name,
            kind: routes::kind(item.kind).into(),
            visibility: format!("{:?}", item.visibility).to_lowercase(),
            property: item.property.as_ref().map(|property| {
                let accessor = |accessor: &AccessorCapability| api::Accessor {
                    visibility: format!("{:?}", accessor.visibility).to_lowercase(),
                    hidden: accessor.hidden,
                };
                api::Property {
                    getter: accessor(&property.getter),
                    setter: property.setter.as_ref().map(accessor),
                }
            }),
            signature: item
                .signature
                .iter()
                .map(|token| token.text.as_str())
                .collect(),
            receiver: item.receiver.as_ref().map(|token| token.text.clone()),
            equality_requirements: item.equality_requirements.as_ref().map(|requirements| {
                requirements
                    .components
                    .iter()
                    .map(|token| token.text.clone())
                    .collect()
            }),
            aliases: item.aliases.clone(),
            source: api::Source {
                path: source.map(|source| source.path.clone()),
                line: item.location.line,
                byte_start: item.location.range.start,
                byte_end: item.location.range.end,
                url: source.map(|source| {
                    format!(
                        "{}#L{}",
                        routes::source(&cone.key, &source.key),
                        item.location.line
                    )
                }),
            },
            url: self.catalog.routes[&item.key].url(),
            documentation: item.comment.as_ref().map(|comment| api::Documentation {
                markdown: comment.body.text.clone(),
                tags: comment
                    .tags
                    .iter()
                    .map(|tag| api::Tag {
                        kind: format!("{:?}", tag.kind).to_lowercase(),
                        label: tag.label.clone(),
                        markdown: tag.body.text.clone(),
                    })
                    .collect(),
            }),
            relations: item
                .relations
                .iter()
                .filter(|relation| self.catalog.relation_visible(cone, relation))
                .map(|relation| api::Relation {
                    kind: format!("{:?}", relation.kind).to_lowercase(),
                    signature: relation.signature.clone(),
                    origin: relation.origin.clone(),
                    url: self.catalog.declaration_url(&relation.target),
                })
                .collect(),
        }
    }
}
