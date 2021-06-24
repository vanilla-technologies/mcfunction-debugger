use crate::parser::{commands::EntityAnchor, Line};
use std::{collections::HashMap, iter::FromIterator};

pub struct TemplateEngine<'l> {
    pub replacements: HashMap<&'l str, &'l str>,
}

impl<'l> TemplateEngine<'l> {
    pub fn extend<T: IntoIterator<Item = (&'l str, &'l str)>>(
        &self,
        iter: T,
    ) -> TemplateEngine<'l> {
        let mut replacements = HashMap::from_iter(iter);
        replacements.extend(self.replacements.iter());
        TemplateEngine { replacements }
    }

    pub fn expand(&self, string: &str) -> String {
        let mut result = string.to_owned();
        for (from, to) in &self.replacements {
            result = result.replace(from, to);
        }
        result
    }

    pub fn expand_line(
        &self,
        (line_number, line, command): &(usize, String, Line),
        namespace: &str,
    ) -> String {
        match command {
            Line::Breakpoint => {
                let template = include_str!(
                    "../datapack_template/data/template/functions/set_breakpoint.mcfunction"
                );
                let template = template.replace("-line_number-", &line_number.to_string());
                self.expand(&template)
            }
            Line::FunctionCall {
                name,
                anchor,
                execute_as,
            } => {
                let template = include_str!(
                    "../datapack_template/data/template/functions/call_function.mcfunction"
                );
                let function_call = format!("function {}", name);
                let execute = line.strip_suffix(&function_call).unwrap(); //TODO panic!
                let debug_anchor = anchor.map_or("".to_string(), |anchor| {
                    let mut anchor_score = 0;
                    if anchor == EntityAnchor::EYES {
                        anchor_score = 1;
                    }
                    format!(
                        "scoreboard players set current {namespace}_anchor {anchor_score}",
                        namespace = namespace,
                        anchor_score = anchor_score
                    )
                });
                let iterate_as = execute_as
                    .then(|| "iterate")
                    .unwrap_or("iterate_same_executor");
                let template = template
                    .replace("-call_ns-", name.namespace())
                    .replace("-call/fn-", name.name())
                    .replace("execute run ", execute)
                    .replace("# -debug_anchor-", &debug_anchor)
                    .replace("-iterate_as-", iterate_as);
                self.expand(&template)
            }
            Line::OtherCommand => line.to_owned(),
        }
    }
}
