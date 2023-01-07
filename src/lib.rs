// mcfunction-debugger is a debugger for Minecraft's *.mcfunction files that does not require any
// Minecraft mods.
//
// © Copyright (C) 2021-2023 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
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

#[macro_use]
mod macros;

pub mod parser;
pub mod template_engine;
pub mod utils;

use crate::{
    parser::{command::argument::MinecraftEntityAnchor, parse_line, Line},
    template_engine::{exclude_internal_entites_from_selectors, TemplateEngine},
};
use futures::{future::try_join_all, FutureExt};
use multimap::MultiMap;
use parser::command::{resource_location::ResourceLocation, CommandParser};
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    ffi::OsStr,
    fmt::Display,
    fs::File,
    io::{self, BufRead},
    iter::{repeat, FromIterator},
    path::{Path, PathBuf},
    str::FromStr,
};
use tokio::{
    fs::{create_dir_all, write},
    task::JoinHandle,
    try_join,
};
use walkdir::WalkDir;

pub struct Config<'l> {
    pub namespace: &'l str,
    pub shadow: bool,
    pub adapter: Option<AdapterConfig<'l>>,
}
impl Config<'_> {
    fn get_breakpoint_kind(
        &self,
        function: &ResourceLocation,
        line_number: usize,
        after_function: bool,
    ) -> Option<BreakpointKind> {
        if let Some(config) = self.adapter.as_ref() {
            if let Some(vec) = config.breakpoints.get_vec(function) {
                return vec
                    .iter()
                    .filter(|breakpoint| breakpoint.line_number == line_number)
                    .filter(|breakpoint| breakpoint.kind.is_after_function() == after_function)
                    .next()
                    .map(|it| it.kind);
            }
        }
        None
    }
}
pub struct AdapterConfig<'l> {
    pub adapter_listener_name: &'l str,
    pub breakpoints: &'l MultiMap<ResourceLocation, LocalBreakpoint>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LocalBreakpoint {
    pub line_number: usize,
    pub kind: BreakpointKind,
}
impl LocalBreakpoint {
    fn can_resume(&self) -> bool {
        self.kind.can_resume()
    }

    fn get_position(&self) -> Position {
        Position {
            line_number: self.line_number,
            position_in_line: self.kind.get_position_in_line(),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BreakpointKind {
    Normal,
    Invalid,
    Continue { after_function: bool },
    Step { after_function: bool },
}
impl BreakpointKind {
    pub fn can_resume(&self) -> bool {
        match self {
            BreakpointKind::Normal => true,
            BreakpointKind::Invalid => false,
            BreakpointKind::Continue { .. } => true,
            BreakpointKind::Step { .. } => true,
        }
    }

    fn is_after_function(&self) -> bool {
        match self {
            BreakpointKind::Normal => false,
            BreakpointKind::Invalid => false,
            BreakpointKind::Continue { after_function }
            | BreakpointKind::Step { after_function } => *after_function,
        }
    }

    fn get_position_in_line(&self) -> PositionInLine {
        if self.is_after_function() {
            PositionInLine::AfterFunction
        } else {
            PositionInLine::Breakpoint
        }
    }
}

/// Visible for testing only. This is a binary crate, it is not intended to be used as a library.
pub async fn generate_debug_datapack<'l>(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    config: &Config<'l>,
) -> io::Result<()> {
    let functions = find_function_files(input_path).await?;
    let function_contents = parse_functions(&functions, config).await?;

    let output_name = output_path
        .as_ref()
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default();
    let engine = TemplateEngine::new(
        BTreeMap::from_iter([("-ns-", config.namespace), ("-datapack-", output_name)]),
        config
            .adapter
            .as_ref()
            .map(|config| config.adapter_listener_name),
    );
    expand_templates(&engine, &function_contents, &output_path, config).await
}

async fn find_function_files(
    datapack_path: impl AsRef<Path>,
) -> Result<BTreeMap<ResourceLocation, PathBuf>, io::Error> {
    let data_path = datapack_path.as_ref().join("data");
    let threads = data_path
        .read_dir()?
        .collect::<io::Result<Vec<_>>>()?
        .into_iter()
        .map(|entry| get_functions(entry).map(|result| result?));

    Ok(try_join_all(threads)
        .await?
        .into_iter()
        .flat_map(|it| it)
        .collect::<BTreeMap<ResourceLocation, PathBuf>>())
}

fn get_functions(
    entry: std::fs::DirEntry,
) -> JoinHandle<Result<Vec<(ResourceLocation, PathBuf)>, io::Error>> {
    tokio::spawn(async move {
        let mut functions = Vec::new();
        if entry.file_type()?.is_dir() {
            let namespace = entry.file_name();
            let namespace_path = entry.path();
            let functions_path = namespace_path.join("functions");
            if functions_path.is_dir() {
                for f_entry in WalkDir::new(&functions_path) {
                    let f_entry = f_entry?;
                    let path = f_entry.path().to_owned();
                    let file_type = f_entry.file_type();
                    if file_type.is_file() {
                        if let Some(extension) = path.extension() {
                            if extension == "mcfunction" {
                                let relative_path = path.strip_prefix(&functions_path).unwrap();
                                let name = ResourceLocation::new(
                                    namespace.to_string_lossy().as_ref(),
                                    &relative_path
                                        .with_extension("")
                                        .to_string_lossy()
                                        .replace(std::path::MAIN_SEPARATOR, "/"),
                                );

                                functions.push((name, path));
                            }
                        }
                    }
                }
            }
        }
        Ok(functions)
    })
}

async fn parse_functions<'l>(
    functions: &'l BTreeMap<ResourceLocation, PathBuf>,
    config: &Config<'_>,
) -> Result<HashMap<&'l ResourceLocation, Vec<(usize, String, Line)>>, io::Error> {
    let parser =
        CommandParser::default().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    functions
        .iter()
        .map(|(name, path)| {
            //TODO async
            let file = File::open(path)?;
            let lines = io::BufReader::new(file)
                .lines()
                .enumerate()
                .map(|(line_number, line)| {
                    line.map(|line| {
                        let command = parse_line(&parser, &line, config.adapter.is_none());
                        (line_number + 1, line, command)
                    })
                })
                .collect::<io::Result<Vec<(usize, String, Line)>>>()?;
            Ok((name, lines))
        })
        .collect()
}

async fn expand_templates(
    engine: &TemplateEngine<'_>,
    function_contents: &HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
    output_path: impl AsRef<Path>,
    config: &Config<'_>,
) -> io::Result<()> {
    try_join!(
        expand_global_templates(engine, function_contents, &output_path, config),
        expand_function_specific_templates(engine, function_contents, &output_path, config),
    )?;
    Ok(())
}

macro_rules! expand_template {
    ($e:expr, $o:expr, $p:expr) => {{
        let path = $o.join($e.expand($p));
        let content = $e.expand(include_template!($p));
        write(path, content)
    }};
}

async fn expand_global_templates(
    engine: &TemplateEngine<'_>,
    function_contents: &HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
    output_path: impl AsRef<Path>,
    config: &Config<'_>,
) -> io::Result<()> {
    let output_path = output_path.as_ref();

    macro_rules! expand {
        ($p:literal) => {
            expand_template!(engine, output_path, $p)
        };
    }

    try_join!(
        create_dir_all(output_path.join(engine.expand("data/-ns-/functions/id"))),
        create_dir_all(output_path.join("data/debug/functions")),
        create_dir_all(output_path.join("data/minecraft/tags/functions")),
    )?;

    try_join!(
        expand!("data/-ns-/functions/id/assign.mcfunction"),
        expand!("data/-ns-/functions/id/init_self.mcfunction"),
        expand!("data/-ns-/functions/id/install.mcfunction"),
        expand!("data/-ns-/functions/id/uninstall.mcfunction"),
        expand!("data/-ns-/functions/abort_session.mcfunction"),
        expand!("data/-ns-/functions/animate_context.mcfunction"),
        expand!("data/-ns-/functions/decrement_age.mcfunction"),
        expand!("data/-ns-/functions/freeze_aec.mcfunction"),
        expand!("data/-ns-/functions/install.mcfunction"),
        expand!("data/-ns-/functions/load.mcfunction"),
        expand!("data/-ns-/functions/on_session_exit_successful.mcfunction"),
        expand!("data/-ns-/functions/on_session_exit.mcfunction"),
        expand!("data/-ns-/functions/reset_skipped.mcfunction"),
        expand!("data/-ns-/functions/resume_immediately.mcfunction"),
        expand_resume_self_template(&engine, function_contents, &output_path, config),
        expand!("data/-ns-/functions/resume_unchecked.mcfunction"),
        expand_schedule_template(&engine, function_contents, &output_path),
        expand!("data/-ns-/functions/select_entity.mcfunction"),
        expand!("data/-ns-/functions/skipped_functions_warning.mcfunction"),
        expand!("data/-ns-/functions/tick_start.mcfunction"),
        expand!("data/-ns-/functions/tick.mcfunction"),
        expand!("data/-ns-/functions/unfreeze_aec.mcfunction"),
        expand!("data/-ns-/functions/uninstall.mcfunction"),
        expand_scores_templates(&engine, function_contents, &output_path),
        expand_validate_all_functions_template(&engine, function_contents, &output_path),
        expand!("data/debug/functions/install.mcfunction"),
        expand!("data/debug/functions/resume.mcfunction"),
        expand!("data/debug/functions/show_scores.mcfunction"),
        expand_show_skipped_template(&engine, function_contents, &output_path),
        expand!("data/debug/functions/stop.mcfunction"),
        expand!("data/debug/functions/uninstall.mcfunction"),
        expand!("data/minecraft/tags/functions/load.json"),
        expand!("data/minecraft/tags/functions/tick.json"),
        expand!("pack.mcmeta"),
    )?;

    Ok(())
}

async fn expand_resume_self_template(
    engine: &TemplateEngine<'_>,
    function_contents: &HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
    output_path: impl AsRef<Path>,
    config: &Config<'_>,
) -> io::Result<()> {
    let mut breakpoints = function_contents
        .iter()
        .flat_map(|(name, lines)| {
            repeat(*name).zip(
                lines
                    .iter()
                    .filter(|(_, _, command)| matches!(command, Line::Breakpoint))
                    .map(|it| Position {
                        line_number: it.0,
                        position_in_line: PositionInLine::Breakpoint,
                    }),
            )
        })
        .collect::<Vec<_>>();

    if let Some(config) = config.adapter.as_ref() {
        // See https://github.com/havarnov/multimap/pull/38
        breakpoints.extend(config.breakpoints.iter_all().flat_map(
            |(function, local_breakpoints)| {
                local_breakpoints
                    .iter()
                    .filter(|breakpoint| breakpoint.can_resume())
                    .map(move |breakpoint| (function, breakpoint.get_position()))
            },
        ));
    }

    let resume_cases = breakpoints
        .into_iter()
        .map(|(name, position)| {
            engine.expand(&format!(
                "execute \
                if entity @s[tag=-ns-+{original_namespace}+{original_function_tag}+{position}] \
                run function -ns-:{original_namespace}/{original_function}/\
                continue_current_iteration_at_{position}",
                original_namespace = name.namespace(),
                original_function = name.path(),
                original_function_tag = name.path().replace("/", "+"),
                position = position,
            ))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let engine = engine.extend([("# -resume_cases-", resume_cases.as_str())]);
    let path = output_path.as_ref();
    expand_template!(engine, path, "data/-ns-/functions/resume_self.mcfunction").await
}

async fn expand_schedule_template(
    engine: &TemplateEngine<'_>,
    function_contents: &HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
    output_path: impl AsRef<Path>,
) -> io::Result<()> {
    #[rustfmt::skip]
    macro_rules! PATH { () => { "data/-ns-/functions/schedule.mcfunction" }; }

    let content = function_contents
        .keys()
        .map(|name| {
            let engine = engine.extend_orig_name(name);
            engine.expand(include_template!(PATH!()))
        })
        .collect::<Vec<_>>()
        .join("");

    let path = output_path.as_ref().join(engine.expand(PATH!()));
    write(&path, &content).await
}

async fn expand_scores_templates(
    engine: &TemplateEngine<'_>,
    function_contents: &HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
    output_path: impl AsRef<Path>,
) -> io::Result<()> {
    let objectives = function_contents
        .values()
        .flat_map(|vec| vec)
        .filter_map(|(_, _, line)| line.objectives())
        .flat_map(|objectives| objectives)
        .collect::<BTreeSet<_>>();

    try_join!(
        expand_log_scores_template(&objectives, engine, &output_path),
        expand_update_scores_template(&objectives, engine, &output_path),
    )?;

    Ok(())
}

async fn expand_log_scores_template(
    objectives: &BTreeSet<&String>,
    engine: &TemplateEngine<'_>,
    output_path: impl AsRef<Path>,
) -> Result<(), io::Error> {
    #[rustfmt::skip]
    macro_rules! PATH { () => { "data/-ns-/functions/log_scores.mcfunction" }; }

    let content = objectives
        .iter()
        .map(|objective| {
            let engine = engine.extend([("-objective-", objective.as_str())]);
            engine.expand(include_template!(PATH!()))
        })
        .collect::<Vec<_>>()
        .join("");
    let path = output_path.as_ref().join(engine.expand(PATH!()));
    write(&path, &content).await
}

async fn expand_update_scores_template(
    objectives: &BTreeSet<&String>,
    engine: &TemplateEngine<'_>,
    output_path: impl AsRef<Path>,
) -> Result<(), io::Error> {
    #[rustfmt::skip]
    macro_rules! PATH { () => { "data/-ns-/functions/update_scores.mcfunction" }; }

    let content = objectives
        .iter()
        .map(|objective| {
            let engine = engine.extend([("-objective-", objective.as_str())]);
            engine.expand(include_template!(PATH!()))
        })
        .collect::<Vec<_>>()
        .join("");
    let path = output_path.as_ref().join(engine.expand(PATH!()));
    write(&path, &content).await
}

async fn expand_validate_all_functions_template(
    engine: &TemplateEngine<'_>,
    function_contents: &HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
    output_path: impl AsRef<Path>,
) -> io::Result<()> {
    #[rustfmt::skip]
    macro_rules! PATH { () => { "data/-ns-/functions/validate_all_functions.mcfunction" }; }

    let content = function_contents
        .keys()
        .map(|name| {
            let engine = engine.extend_orig_name(name);
            engine.expand(include_template!(PATH!()))
        })
        .collect::<Vec<_>>()
        .join("");

    let path = output_path.as_ref().join(engine.expand(PATH!()));
    write(&path, &content).await
}

async fn expand_show_skipped_template(
    engine: &TemplateEngine<'_>,
    function_contents: &HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
    output_path: impl AsRef<Path>,
) -> io::Result<()> {
    // This may include calls to non-existent functions
    let called_functions = function_contents
        .values()
        .flat_map(|vec| vec)
        .filter_map(|(_, _, line)| match line {
            Line::FunctionCall { name, .. } => Some(name),
            _ => None,
        })
        .collect::<BTreeSet<_>>();

    let execute_if_skipped = "execute if score -orig_ns-:-orig/fn- -ns-_skipped matches 1..";
    let is_valid = "score -orig_ns-:-orig/fn- -ns-_valid matches 0";
    let tellraw = r#"tellraw @s [{"text":" - -orig_ns-:-orig/fn- ("},{"score":{"name":"-orig_ns-:-orig/fn-","objective":"-ns-_skipped"}},{"text":"x)"}]"#;

    let missing_functions = called_functions
        .iter()
        .map(|orig_name| {
            engine.extend_orig_name(orig_name).expand(&format!(
                "{} {} {} run {}",
                execute_if_skipped, "unless", is_valid, tellraw
            ))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let invalid_functions = called_functions
        .iter()
        .map(|orig_name| {
            engine.extend_orig_name(orig_name).expand(&format!(
                "{} {} {} run {}",
                execute_if_skipped, "if", is_valid, tellraw
            ))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let output_path = output_path.as_ref();
    expand_template!(
        engine.extend([
            ("# -missing_functions-", missing_functions.as_str()),
            ("# -invalid_functions-", invalid_functions.as_str()),
        ]),
        output_path,
        "data/debug/functions/show_skipped.mcfunction"
    )
    .await?;

    Ok(())
}

async fn expand_function_specific_templates(
    engine: &TemplateEngine<'_>,
    function_contents: &HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
    output_path: impl AsRef<Path>,
    config: &Config<'_>,
) -> io::Result<()> {
    let call_tree = create_call_tree(&function_contents);

    try_join_all(function_contents.iter().map(|(fn_name, lines)| {
        expand_function_templates(&engine, fn_name, lines, &call_tree, &output_path, config)
    }))
    .await?;

    Ok(())
}

fn create_call_tree<'l>(
    function_contents: &'l HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
) -> MultiMap<&'l ResourceLocation, (&'l ResourceLocation, &'l usize)> {
    function_contents
        .iter()
        .flat_map(|(&caller, lines)| {
            lines
                .iter()
                .filter_map(move |(line_number, _line, command)| {
                    if let Line::FunctionCall { name: callee, .. } = command {
                        Some((callee, (caller, line_number)))
                    } else {
                        None
                    }
                })
        })
        .collect()
}

async fn expand_function_templates(
    engine: &TemplateEngine<'_>,
    fn_name: &ResourceLocation,
    lines: &Vec<(usize, String, Line)>,
    call_tree: &MultiMap<&ResourceLocation, (&ResourceLocation, &usize)>,
    output_path: impl AsRef<Path>,
    config: &Config<'_>,
) -> io::Result<()> {
    let engine = engine.extend_orig_name(fn_name);

    let output_path = output_path.as_ref();
    let fn_dir = output_path.join(engine.expand("data/-ns-/functions/-orig_ns-/-orig/fn-"));
    create_dir_all(&fn_dir).await?;

    let partitions = partition(fn_name, lines, config);

    let mut first = true;
    for (partition_index, partition) in partitions.iter().enumerate() {
        let position = partition.start.to_string();
        let positions = format!("{}-{}", partition.start, partition.end);
        let engine = engine.extend([
            ("-position-", position.as_str()),
            ("-positions-", positions.as_str()),
        ]);
        macro_rules! expand {
            ($p:literal) => {
                expand_template!(engine, output_path, $p)
            };
        }

        if first {
            expand!("data/-ns-/functions/-orig_ns-/-orig/fn-/next_iteration_or_return.mcfunction")
                .await?;
            first = false;
        } else {
            expand!(
                "data/-ns-/functions/-orig_ns-/-orig/fn-/continue_current_iteration_at_-position-.mcfunction"
            )
            .await?;
        }

        // continue_at_-position-.mcfunction
        #[rustfmt::skip]
        macro_rules! PATH { () => {"data/-ns-/functions/-orig_ns-/-orig/fn-/continue_at_-position-.mcfunction"} }
        let path = output_path.join(engine.expand(PATH!()));
        let template = include_template!(PATH!()).to_string();
        write(&path, &engine.expand(&template)).await?;

        // -positions-.mcfunction
        let mut content = partition
            .regular_lines
            .iter()
            .map(|line| engine.expand_line(line))
            .collect::<Vec<_>>()
            .join("\n");

        let terminator = match partition.terminator {
            Terminator::Breakpoint => {
                expand_breakpoint_template(partition, &engine, StoppedReason::Breakpoint, 0)
            }
            Terminator::StepBeforeFunction => {
                expand_breakpoint_template(partition, &engine, StoppedReason::Step, 0)
            }
            Terminator::StepAfterFunction => {
                let (_line_number, line, _parsed) = &lines[partition.end.line_number - 1];
                let column = line.len() + 1;
                expand_breakpoint_template(partition, &engine, StoppedReason::Step, column)
            }
            Terminator::ContinueBeforeFunction | Terminator::ContinueAfterFunction => {
                let next_partition = &partitions[partition_index + 1];
                let next_positions = format!("{}-{}", next_partition.start, next_partition.end);
                let engine = engine.extend([("-next_positions-", next_positions.as_str())]);
                engine.expand(&format!(
                    "function -ns-:-orig_ns-/-orig/fn-/-next_positions-"
                ))
            }
            Terminator::FunctionCall {
                line,
                name,
                anchor,
                selectors,
            } => {
                let line_number = (partition.end.line_number).to_string();
                let line = exclude_internal_entites_from_selectors(line, selectors);
                let function_call = format!("function {}", name);
                let execute = line.strip_suffix(&function_call).unwrap(); // TODO panic!
                let debug_anchor = anchor.map_or("".to_string(), |anchor| {
                    let mut anchor_score = 0;
                    if anchor == MinecraftEntityAnchor::EYES {
                        anchor_score = 1;
                    }
                    format!(
                        "execute if score -orig_ns-:-orig/fn- -ns-_valid matches 1 run \
                            scoreboard players set current -ns-_anchor {anchor_score}",
                        anchor_score = anchor_score
                    )
                });
                let engine = engine.extend([
                    ("-line_number-", line_number.as_str()),
                    ("-call_ns-", name.namespace()),
                    ("-call/fn-", name.path()),
                    ("execute run ", execute),
                    ("# -debug_anchor-", &debug_anchor),
                ]);
                let template =
                    include_template!("data/template/functions/call_function.mcfunction");
                engine.expand(&template)
            }
            Terminator::Return => {
                let template = include_template!("data/template/functions/return.mcfunction");
                engine.expand(&template)
            }
        };
        content.push('\n');
        content.push_str(&terminator);

        expand_template!(
            engine.extend([("# -content-", content.as_str())]),
            output_path,
            "data/-ns-/functions/-orig_ns-/-orig/fn-/-positions-.mcfunction"
        )
        .await?;
    }

    macro_rules! expand {
        ($p:literal) => {
            expand_template!(engine, output_path, $p)
        };
    }

    create_parent_dir(output_path.join(engine.expand("data/debug/functions/-orig_ns-/-orig/fn-")))
        .await?;

    try_join!(
        expand!("data/-ns-/functions/-orig_ns-/-orig/fn-/return.mcfunction"),
        expand!("data/-ns-/functions/-orig_ns-/-orig/fn-/return_or_exit.mcfunction"),
        expand!("data/-ns-/functions/-orig_ns-/-orig/fn-/scheduled.mcfunction"),
        expand!("data/-ns-/functions/-orig_ns-/-orig/fn-/start.mcfunction"),
        expand!("data/-ns-/functions/-orig_ns-/-orig/fn-/start_valid.mcfunction"),
        expand!("data/debug/functions/-orig_ns-/-orig/fn-.mcfunction"),
    )?;

    if config.shadow {
        create_parent_dir(output_path.join(engine.expand("data/-orig_ns-/functions/-orig/fn-")))
            .await?;
        expand!("data/-orig_ns-/functions/-orig/fn-.mcfunction").await?;
    }

    if let Some(callers) = call_tree.get_vec(fn_name) {
        let mut return_cases = callers
            .iter()
            .map(|(caller, line_number)| {
                engine.expand(&format!(
                    "execute if entity \
                    @s[tag=-ns-+{caller_namespace}+{caller_function_tag}+{line_number}] run \
                    function -ns-:{caller_namespace}/{caller_function}/\
                    continue_current_iteration_at_{line_number}_function",
                    caller_namespace = caller.namespace(),
                    caller_function = caller.path(),
                    caller_function_tag = caller.path().replace("/", "+"),
                    line_number = line_number
                ))
            })
            .collect::<Vec<_>>();
        return_cases.sort();
        let return_cases = return_cases.join("\n");

        expand_template!(
            engine.extend([("# -return_cases-", return_cases.as_str())]),
            output_path,
            "data/-ns-/functions/-orig_ns-/-orig/fn-/return_self.mcfunction"
        )
        .await?;
    }

    let commands = lines
        .iter()
        .map(|(_, line, parsed)| match parsed {
            Line::Empty | Line::Comment | Line::Breakpoint => line.to_string(),
            _ => {
                format!(
                    "execute if score 1 -ns-_constant matches 0 run {}",
                    line.trim_start()
                )
            }
        })
        .collect::<Vec<_>>()
        .join("\n");
    expand_template!(
        engine.extend([("# -commands-", commands.as_str())]),
        output_path,
        "data/-ns-/functions/-orig_ns-/-orig/fn-/validate.mcfunction"
    )
    .await?;

    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum StoppedReason {
    Breakpoint,
    Step,
}
impl FromStr for StoppedReason {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "breakpoint" => Ok(StoppedReason::Breakpoint),
            "step" => Ok(StoppedReason::Step),
            _ => Err(()),
        }
    }
}
impl Display for StoppedReason {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StoppedReason::Breakpoint => write!(f, "breakpoint"),
            StoppedReason::Step => write!(f, "step"),
        }
    }
}

fn expand_breakpoint_template(
    partition: &Partition,
    engine: &TemplateEngine,
    reason: StoppedReason,
    column: usize,
) -> String {
    let line_number = partition.end.line_number.to_string();
    let position = partition.end.to_string();
    let reason = reason.to_string();
    let column_str = &format!(":{}", column);
    let optional_column = if column == 0 { "" } else { column_str };
    let engine = engine.extend([
        ("-line_number-", line_number.as_str()),
        ("-position-", &position),
        ("-reason-", &reason),
        ("-optional_column-", optional_column),
    ]);
    let template = include_template!("data/template/functions/set_breakpoint.mcfunction");
    engine.expand(&template)
}

struct Partition<'l> {
    start: Position,
    end: Position,
    regular_lines: &'l [(usize, String, Line)],
    terminator: Terminator<'l>,
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
struct Position {
    line_number: usize,
    position_in_line: PositionInLine,
}
impl FromStr for Position {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        fn from_str_inner(s: &str) -> Option<Position> {
            let (line_number, position_in_line) = s.split_once('_')?;
            let line_number = line_number.parse().ok()?;
            let position_in_line = position_in_line.parse().ok()?;
            Some(Position {
                line_number,
                position_in_line,
            })
        }
        from_str_inner(s).ok_or(())
    }
}
impl Display for Position {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}_{}", self.line_number, self.position_in_line)
    }
}
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PositionInLine {
    Entry,
    Breakpoint,
    Function,
    AfterFunction,
    Return,
}
impl FromStr for PositionInLine {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "entry" => Ok(PositionInLine::Entry),
            "breakpoint" => Ok(PositionInLine::Breakpoint),
            "function" => Ok(PositionInLine::Function),
            "after_function" => Ok(PositionInLine::AfterFunction),
            "return" => Ok(PositionInLine::Return),
            _ => Err(()),
        }
    }
}
impl Display for PositionInLine {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PositionInLine::Entry => write!(f, "entry"),
            PositionInLine::Breakpoint => write!(f, "breakpoint"),
            PositionInLine::Function => write!(f, "function"),
            PositionInLine::AfterFunction => write!(f, "after_function"),
            PositionInLine::Return => write!(f, "return"),
        }
    }
}
enum Terminator<'l> {
    Breakpoint,
    ContinueBeforeFunction,
    ContinueAfterFunction,
    StepBeforeFunction,
    StepAfterFunction,
    FunctionCall {
        line: &'l str,
        name: &'l ResourceLocation,
        anchor: &'l Option<MinecraftEntityAnchor>,
        selectors: &'l BTreeSet<usize>,
    },
    Return,
}
impl Terminator<'_> {
    fn get_position_in_line(&self) -> PositionInLine {
        match self {
            Terminator::Breakpoint => PositionInLine::Breakpoint,
            Terminator::ContinueBeforeFunction => PositionInLine::Breakpoint,
            Terminator::ContinueAfterFunction => PositionInLine::AfterFunction,
            Terminator::StepBeforeFunction => PositionInLine::Breakpoint,
            Terminator::StepAfterFunction => PositionInLine::AfterFunction,
            Terminator::FunctionCall { .. } => PositionInLine::Function,
            Terminator::Return => PositionInLine::Return,
        }
    }
}

fn partition<'l>(
    fn_name: &ResourceLocation,
    lines: &'l [(usize, String, Line)],
    config: &Config,
) -> Vec<Partition<'l>> {
    let mut partitions = Vec::new();
    let mut start_line_index = 0;
    let mut start = Position {
        line_number: 0,
        position_in_line: PositionInLine::Entry,
    };
    // TODO: Can we remove line_number from the triple?
    for (line_index, (_line_number, line, command)) in lines.iter().enumerate() {
        let line_number = line_index + 1;
        let mut next_partition = |terminator: Terminator<'l>| {
            let end = Position {
                line_number,
                position_in_line: terminator.get_position_in_line(),
            };
            let partition = Partition {
                start,
                end,
                regular_lines: &lines[start_line_index..line_index],
                terminator,
            };
            start = end;
            start_line_index = line_index;
            partition
        };
        match config.get_breakpoint_kind(fn_name, line_number, false) {
            Some(BreakpointKind::Normal) => {
                partitions.push(next_partition(Terminator::Breakpoint));
            }
            Some(BreakpointKind::Invalid) => {}
            Some(BreakpointKind::Continue {
                after_function: false,
            }) => {
                partitions.push(next_partition(Terminator::ContinueBeforeFunction));
            }
            Some(BreakpointKind::Continue {
                after_function: true,
            }) => {} // Is added after function
            Some(BreakpointKind::Step {
                after_function: false,
            }) => {
                partitions.push(next_partition(Terminator::StepBeforeFunction));
            }
            Some(BreakpointKind::Step {
                after_function: true,
            }) => {} // Is added after function
            None => {}
        }
        if matches!(command, Line::Breakpoint) {
            partitions.push(next_partition(Terminator::Breakpoint));
        }
        if let Line::FunctionCall {
            name,
            anchor,
            selectors,
            ..
        } = command
        {
            partitions.push(next_partition(Terminator::FunctionCall {
                line,
                name,
                anchor,
                selectors,
            }));
        }

        if let Some(BreakpointKind::Continue {
            after_function: true,
        }) = config.get_breakpoint_kind(fn_name, line_number, true)
        {
            partitions.push(next_partition(Terminator::ContinueAfterFunction));
        }
        if let Some(BreakpointKind::Step {
            after_function: true,
        }) = config.get_breakpoint_kind(fn_name, line_number, true)
        {
            partitions.push(next_partition(Terminator::StepAfterFunction));
        }

        if matches!(command, Line::Breakpoint | Line::FunctionCall { .. }) {
            start_line_index += 1; // Skip the line containing the breakpoint / function call
        }
    }
    partitions.push(Partition {
        start,
        end: Position {
            line_number: lines.len(),
            position_in_line: PositionInLine::Return,
        },
        regular_lines: &lines[start_line_index..lines.len()],
        terminator: Terminator::Return,
    });
    partitions
}

async fn create_parent_dir(path: impl AsRef<Path>) -> io::Result<()> {
    if let Some(parent_dir) = path.as_ref().parent() {
        create_dir_all(parent_dir).await?;
    }
    Ok(())
}
