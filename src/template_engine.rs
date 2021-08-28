use crate::parser::{
    commands::{MinecraftEntityAnchor, NamespacedNameRef},
    Line,
};
use std::{collections::HashMap, iter::FromIterator};

pub struct TemplateEngine<'l> {
    replacements: HashMap<&'l str, &'l str>,
    replacements_owned: HashMap<&'l str, String>,
}

impl<'l> TemplateEngine<'l> {
    pub fn new(replacements: HashMap<&'l str, &'l str>) -> TemplateEngine<'l> {
        TemplateEngine {
            replacements,
            replacements_owned: HashMap::new(),
        }
    }

    pub fn extend<T: IntoIterator<Item = (&'l str, &'l str)>>(
        &self,
        iter: T,
    ) -> TemplateEngine<'l> {
        let mut replacements = HashMap::from_iter(iter);
        replacements.extend(self.replacements.iter());
        TemplateEngine {
            replacements,
            replacements_owned: self.replacements_owned.clone(),
        }
    }

    pub fn extend_orig_name<N: AsRef<str>>(
        &'l self,
        orig_name: &'l NamespacedNameRef<N>,
    ) -> TemplateEngine<'l> {
        let orig_fn_tag = orig_name.name().replace('/', "_");
        let mut engine = self.extend([
            ("-orig_ns-", orig_name.namespace()),
            ("-orig/fn-", orig_name.name()),
        ]);
        engine.replacements_owned.insert("-orig_fn-", orig_fn_tag);
        engine
    }

    pub fn expand(&self, string: &str) -> String {
        let mut result = string.to_owned();
        for (from, to) in &self.replacements {
            result = result.replace(from, to);
        }
        for (from, to) in &self.replacements_owned {
            result = result.replace(from, to);
        }
        result
    }

    pub fn expand_line(&self, (line_number, line, command): &(usize, String, Line)) -> String {
        match command {
            Line::Breakpoint => {
                let template = include_str!(
                    "datapack_template/data/template/functions/set_breakpoint.mcfunction"
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
                    "datapack_template/data/template/functions/call_function.mcfunction"
                );
                let function_call = format!("function {}", name);
                let execute = line.strip_suffix(&function_call).unwrap(); //TODO panic!
                let debug_anchor = anchor.map_or("".to_string(), |anchor| {
                    let mut anchor_score = 0;
                    if anchor == MinecraftEntityAnchor::EYES {
                        anchor_score = 1;
                    }
                    format!(
                        "scoreboard players set current -ns-_anchor {anchor_score}",
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
            Line::Schedule {
                schedule_start,
                function,
                time,
                category,
            } => {
                let template = if *category == Some("append".to_string()) {
                    include_str!(
                        "datapack_template/data/template/functions/schedule_append.mcfunction"
                    )
                } else {
                    if *category == Some("clear".to_string()) {
                        include_str!(
                            "datapack_template/data/template/functions/schedule_clear.mcfunction"
                        )
                    } else {
                        include_str!(
                            "datapack_template/data/template/functions/schedule_replace.mcfunction"
                        )
                    }
                };
                let schedule_fn = function.name().replace('/', "_");
                let mut engine = self.extend([
                    ("-schedule_ns-", function.namespace()),
                    ("-schedule_fn-", &schedule_fn),
                    ("execute run ", &line[..*schedule_start]),
                ]);

                let ticks;
                if let Some(time) = time {
                    ticks = time.as_ticks().to_string();
                    engine = engine.extend([("-ticks-", ticks.as_str())]);
                }
                engine.expand(template)
            }
            Line::OtherCommand { selectors } => {
                let mut remaining_line = line.as_str();
                let mut result = String::new();
                for selector in selectors {
                    const MIN_SELECTOR_LEN: usize = "@e".len();
                    let (prefix, suffix) = remaining_line.split_at(selector + MIN_SELECTOR_LEN);
                    remaining_line = suffix;
                    result.push_str(prefix);

                    let trivial_selector = !remaining_line.starts_with('[');
                    remaining_line = remaining_line.strip_prefix('[').unwrap_or(remaining_line);
                    result.push_str("[tag=!-ns-");
                    if trivial_selector {
                        result.push(']');
                    } else {
                        result.push(',');
                    }
                }
                result.push_str(remaining_line);
                self.expand(&result)
            }
        }
    }
}
