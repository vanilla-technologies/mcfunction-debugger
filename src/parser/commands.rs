use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn default_commands() -> serde_json::Result<HashMap<String, CommandsNode>> {
    let data = include_str!("commands.json");
    let root_node: RootNode = serde_json::from_str(data)?;
    Ok(root_node.children)
}

#[derive(Deserialize, Serialize, Debug)]
#[serde(tag = "type", rename = "root")]
struct RootNode {
    children: HashMap<String, CommandsNode>,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum CommandsNode {
    Literal {
        #[serde(flatten)]
        node: Node,
    },
    Argument {
        #[serde(flatten)]
        node: Node,
        parser: ArgumentParser,
    },
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Node {
    #[serde(default)]
    pub children: HashMap<String, CommandsNode>,
    #[serde(default)]
    pub executable: bool,
    #[serde(default)]
    pub redirect: Vec<String>,
}

impl CommandsNode {
    pub fn children(&self) -> &HashMap<String, CommandsNode> {
        match self {
            CommandsNode::Literal { node, .. } => &node.children,
            CommandsNode::Argument { node, .. } => &node.children,
        }
    }

    pub fn executable(&self) -> bool {
        match self {
            CommandsNode::Literal { node, .. } => node.executable,
            CommandsNode::Argument { node, .. } => node.executable,
        }
    }
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ArgumentParser {
    #[serde(rename = "minecraft:swizzle")]
    MinecraftSwizzle,
    #[serde(other, rename = "")]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_json() {
        // when:
        let actual = default_commands().unwrap();

        // then:
        assert!(
            actual.contains_key("execute"),
            "Expected actual to contain key 'execute': {:#?}",
            actual
        );
    }

    #[test]
    fn test_() {
        // when:
        let root = RootNode {
            children: HashMap::new(),
        };

        let actual = serde_json::to_string(&root).unwrap();

        // then:
        assert_eq!(actual, r#"{"type":"root","children":{}}"#);
    }
}
