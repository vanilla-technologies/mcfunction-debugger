use const_format::concatcp;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, fmt::Display};

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

    pub fn redirect(&self) -> Result<Option<&String>, String> {
        let redirect = match self {
            CommandsNode::Literal { node, .. } => &node.redirect,
            CommandsNode::Argument { node, .. } => &node.redirect,
        };
        if redirect.len() > 1 {
            Err(format!("Multi redirect is not supported: {:?}", redirect))
        } else {
            Ok(redirect.first())
        }
    }
}

type Entity = ();
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum EntityAnchor {
    EYES,
    FEET,
}
type Function<'l> = NamespacedNameRef<&'l str>;
type Swizzle = ();

pub enum Argument<'l> {
    Entity(Entity),
    EntityAnchor(EntityAnchor),
    Function(Function<'l>),
    Swizzle(Swizzle),
}

#[derive(Deserialize, Serialize, Debug)]
pub enum ArgumentParser {
    #[serde(rename = "minecraft:entity")]
    MinecraftEntity,
    #[serde(rename = "minecraft:entity_anchor")]
    MinecraftEntityAnchor,
    #[serde(rename = "minecraft:function")]
    MinecraftFunction,
    #[serde(rename = "minecraft:swizzle")]
    MinecraftSwizzle,
    #[serde(other, rename = "")]
    Unknown,
}

impl ArgumentParser {
    pub fn parse<'l>(&self, string: &'l str) -> Result<(Argument<'l>, &'l str), String> {
        match self {
            ArgumentParser::MinecraftEntity => {
                let (entity, suffix) = ArgumentParser::parse_entity(string)?;
                Ok((Argument::Entity(entity), suffix))
            }
            ArgumentParser::MinecraftEntityAnchor => {
                let (entity_anchor, suffix) = ArgumentParser::parse_entity_anchor(string)?;
                Ok((Argument::EntityAnchor(entity_anchor), suffix))
            }
            ArgumentParser::MinecraftFunction => {
                let (function, suffix) = ArgumentParser::parse_function(string)?;
                Ok((Argument::Function(function), suffix))
            }
            ArgumentParser::MinecraftSwizzle => {
                let (swizzle, suffix) = ArgumentParser::parse_swizzle(string)?;
                Ok((Argument::Swizzle(swizzle), suffix))
            }
            ArgumentParser::Unknown => Err("Unknown argument".to_string()),
        }
    }

    // TODO support ] in strings and NBT
    // TODO support for player name and UUID
    // TODO add support for limits on amount and type
    fn parse_entity(mut string: &str) -> Result<(Entity, &str), String> {
        string = string
            .strip_prefix('@')
            .ok_or(format!("Invalid entity {}", string))?;

        string = string
            .strip_prefix(&['a', 'e', 'r', 's'][..])
            .ok_or(format!("Unknown selector type '{}'", &string[..2]))?;

        let suffix = if let Some(string) = string.strip_prefix('[') {
            let end = string.find(']').ok_or(format!("Expected end of options"))?;
            &string[1 + end..]
        } else {
            &string
        };
        Ok(((), suffix))
    }

    fn parse_entity_anchor(string: &str) -> Result<(EntityAnchor, &str), String> {
        let eyes = "eyes";
        let feet = "feet";
        if string.starts_with(eyes) {
            Ok((EntityAnchor::EYES, &string[eyes.len()..]))
        } else if string.starts_with(feet) {
            Ok((EntityAnchor::FEET, &string[feet.len()..]))
        } else {
            Err(format!("Invalid entity anchor {}", string))
        }
    }

    fn parse_function(string: &str) -> Result<(Function, &str), String> {
        let namespace_end = string
            .find(|c| !NAMESPACE_CHARS.contains(c))
            .ok_or(format!("Invalid ID: '{}'", string))?;
        let (_namespace, rest) = string.split_at(namespace_end);
        let rest = rest
            .strip_prefix(':')
            .ok_or(format!("Invalid ID: '{}'", string))?;
        let name_end = rest.find(|c| !NAME_CHARS.contains(c)).unwrap_or(rest.len());
        let len = namespace_end + 1 + name_end;
        let (string, rest) = string.split_at(len);
        let name = NamespacedNameRef {
            string,
            namespace_len: namespace_end,
        };
        Ok((name, rest))
    }

    fn parse_swizzle(string: &str) -> Result<(Swizzle, &str), String> {
        let end = string
            .find(' ')
            .ok_or("Failed to parse swizzle".to_string())?;
        let swizzle = ();
        Ok((swizzle, &string[end..]))
    }
}

const NAMESPACE_CHARS: &str = "0123456789abcdefghijklmnopqrstuvwxyz_-.";
const NAME_CHARS: &str = concatcp!(NAMESPACE_CHARS, "/");

pub type NamespacedName = NamespacedNameRef<String>;

#[derive(Debug, Eq, Hash, PartialEq)]
pub struct NamespacedNameRef<S: AsRef<str>> {
    string: S,
    namespace_len: usize,
}

impl<S: AsRef<str>> NamespacedNameRef<S> {
    pub fn new(namespace: &str, name: &str) -> NamespacedName {
        NamespacedNameRef {
            string: format!("{}:{}", namespace, name),
            namespace_len: namespace.len(),
        }
    }

    pub fn from(string: S) -> Option<NamespacedNameRef<S>> {
        let namespace_len = string.as_ref().find(':')?;
        Some(NamespacedNameRef {
            string,
            namespace_len,
        })
    }

    pub fn namespace(&self) -> &str {
        &self.string.as_ref()[..self.namespace_len]
    }

    pub fn name(&self) -> &str {
        &self.string.as_ref()[self.namespace_len + 1..]
    }

    pub fn to_owned(&self) -> NamespacedName {
        NamespacedName {
            string: self.string.as_ref().to_owned(),
            namespace_len: self.namespace_len,
        }
    }
}

impl<S: AsRef<str>> Display for NamespacedNameRef<S> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.string.as_ref().fmt(f)
    }
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
