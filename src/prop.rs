use std::collections::HashMap;
use std::sync::Mutex;

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DeadPropertyName {
    pub namespace: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DeadProperty {
    pub name: DeadPropertyName,
    pub value: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PropPatchOp {
    Set(DeadProperty),
    Remove(DeadPropertyName),
}

#[derive(Default)]
pub struct PropStore {
    props: Mutex<HashMap<String, HashMap<DeadPropertyName, String>>>,
}

impl PropStore {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn apply(&self, href_path: &str, ops: &[PropPatchOp]) {
        let mut props = self.props.lock().expect("prop store should not poison");
        let path_props = props.entry(href_path.to_owned()).or_default();

        for op in ops {
            match op {
                PropPatchOp::Set(property) => {
                    path_props.insert(property.name.clone(), property.value.clone());
                }
                PropPatchOp::Remove(name) => {
                    path_props.remove(name);
                }
            }
        }

        if path_props.is_empty() {
            props.remove(href_path);
        }
    }

    pub fn get(&self, href_path: &str) -> Vec<DeadProperty> {
        let props = self.props.lock().expect("prop store should not poison");
        props
            .get(href_path)
            .map(|path_props| {
                path_props
                    .iter()
                    .map(|(name, value)| DeadProperty {
                        name: name.clone(),
                        value: value.clone(),
                    })
                    .collect()
            })
            .unwrap_or_default()
    }
}

pub fn parse_proppatch(input: &str) -> Result<Vec<PropPatchOp>, roxmltree::Error> {
    let document = roxmltree::Document::parse(input)?;
    let mut ops = Vec::new();

    for action in document
        .root_element()
        .children()
        .filter(|node| node.is_element())
    {
        let is_set = action.tag_name().name() == "set";
        let is_remove = action.tag_name().name() == "remove";
        if !is_set && !is_remove {
            continue;
        }

        for prop in action
            .children()
            .filter(|node| node.is_element() && node.tag_name().name() == "prop")
        {
            for property in prop.children().filter(|node| node.is_element()) {
                let name = DeadPropertyName {
                    namespace: property
                        .tag_name()
                        .namespace()
                        .unwrap_or_default()
                        .to_owned(),
                    name: property.tag_name().name().to_owned(),
                };

                if is_set {
                    ops.push(PropPatchOp::Set(DeadProperty {
                        name,
                        value: property.text().unwrap_or_default().to_owned(),
                    }));
                } else {
                    ops.push(PropPatchOp::Remove(name));
                }
            }
        }
    }

    Ok(ops)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_set_and_remove_ops() {
        let ops = parse_proppatch(
            r#"<D:propertyupdate xmlns:D="DAV:" xmlns:X="urn:test"><D:set><D:prop><X:color>blue</X:color></D:prop></D:set><D:remove><D:prop><X:size/></D:prop></D:remove></D:propertyupdate>"#,
        )
        .expect("xml should parse");

        assert_eq!(ops.len(), 2);
        assert_eq!(
            ops[0],
            PropPatchOp::Set(DeadProperty {
                name: DeadPropertyName {
                    namespace: "urn:test".into(),
                    name: "color".into()
                },
                value: "blue".into()
            })
        );
        assert_eq!(
            ops[1],
            PropPatchOp::Remove(DeadPropertyName {
                namespace: "urn:test".into(),
                name: "size".into()
            })
        );
    }
}
