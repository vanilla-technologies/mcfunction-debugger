// mcfunction-debugger is a debugger for Minecraft's *.mcfunction files that does not require any
// Minecraft mods.
//
// © Copyright (C) 2021 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
//
// This file is part of mcfunction-debugger.
//
// mcfunction-debugger is free software: you can redistribute it and/or modify it under the terms of
// the GNU General Public License as published by the Free Software Foundation, either version 3 of
// the License, or (at your option) any later version.
//
// mcfunction-debugger is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with mcfunction-debugger.
// If not, see <http://www.gnu.org/licenses/>.

use crate::parser::{
    commands::{MinecraftEntityAnchor, NamespacedNameRef},
    Line, ScheduleOperation,
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
                selectors,
            } => {
                let line = exclude_internal_entites_from_selectors(line, selectors);
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
                let template = include_str!(
                    "datapack_template/data/template/functions/call_function.mcfunction"
                );
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
                operation,
                selectors,
            } => {
                let schedule_fn = function.name().replace('/', "_");
                let execute =
                    exclude_internal_entites_from_selectors(&line[..*schedule_start], selectors);
                let mut engine = self.extend([
                    ("-schedule_ns-", function.namespace()),
                    ("-schedule_fn-", &schedule_fn),
                    ("execute run ", &execute),
                ]);

                let ticks;
                if let ScheduleOperation::APPEND { time } | ScheduleOperation::REPLACE { time } =
                    operation
                {
                    ticks = time.as_ticks().to_string();
                    engine = engine.extend([("-ticks-", ticks.as_str())]);
                }

                let template = match operation {
                    ScheduleOperation::APPEND { .. } => include_str!(
                        "datapack_template/data/template/functions/schedule_append.mcfunction"
                    ),
                    ScheduleOperation::CLEAR => include_str!(
                        "datapack_template/data/template/functions/schedule_clear.mcfunction"
                    ),
                    ScheduleOperation::REPLACE { .. } => include_str!(
                        "datapack_template/data/template/functions/schedule_replace.mcfunction"
                    ),
                };

                engine.expand(template)
            }
            Line::OtherCommand { selectors } => {
                let line = exclude_internal_entites_from_selectors(line, selectors);
                self.expand(&line)
            }
        }
    }
}

fn exclude_internal_entites_from_selectors(line: &str, selectors: &[usize]) -> String {
    let mut index = 0;
    let mut result = String::new();
    for selector in selectors {
        const MIN_SELECTOR_LEN: usize = "@e".len();
        let (prefix, remaining_line) = line.split_at(selector + MIN_SELECTOR_LEN);
        result.push_str(&prefix[index..]);
        index = prefix.len();

        result.push_str("[tag=!-ns-");
        if remaining_line.starts_with('[') {
            index += 1;
            result.push(',');
        } else {
            result.push(']');
        }
    }
    result.push_str(&line[index..]);
    result
}
