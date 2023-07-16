// McFunction-Debugger is a debugger for Minecraft's *.mcfunction files that does not require any
// Minecraft mods.
//
// © Copyright (C) 2021-2023 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
//
// This file is part of McFunction-Debugger.
//
// McFunction-Debugger is free software: you can redistribute it and/or modify it under the terms of
// the GNU General Public License as published by the Free Software Foundation, either version 3 of
// the License, or (at your option) any later version.
//
// McFunction-Debugger is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
// without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
// GNU General Public License for more details.
//
// You should have received a copy of the GNU General Public License along with McFunction-Debugger.
// If not, see <http://www.gnu.org/licenses/>.

pub mod config;
pub mod parser;
pub mod partition;
mod template_engine;

use crate::generator::{
    config::GeneratorConfig,
    parser::{
        command::{
            argument::MinecraftEntityAnchor, resource_location::ResourceLocation, CommandParser,
        },
        parse_line, Line,
    },
    partition::{
        partition, BreakpointPositionInLine, LocalBreakpointPosition, Partition, Terminator,
    },
    template_engine::{exclude_internal_entites_from_selectors, TemplateEngine},
};
use futures::{future::try_join_all, FutureExt};
use multimap::MultiMap;
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    ffi::OsStr,
    fs::read_to_string,
    io::{self},
    iter::FromIterator,
    path::{Path, PathBuf},
};
use tokio::{
    fs::{create_dir_all, write},
    task::JoinHandle,
    try_join,
};
use walkdir::WalkDir;

pub struct DebugDatapackMetadata {
    fn_ids: HashMap<ResourceLocation, usize>,
}
impl DebugDatapackMetadata {
    pub fn new(fn_ids: HashMap<ResourceLocation, usize>) -> DebugDatapackMetadata {
        DebugDatapackMetadata { fn_ids }
    }

    fn get_fn_score_holder(&self, fn_name: &ResourceLocation) -> String {
        self.get_score_holder(fn_name, fn_name.to_string(), |id| format!("fn_{}", id))
    }

    pub fn get_breakpoint_score_holder(
        &self,
        fn_name: &ResourceLocation,
        position: &LocalBreakpointPosition,
    ) -> String {
        self.get_score_holder(fn_name, format!("{}_{}", fn_name, position), |id| {
            format!("fn_{}_{}", id, position)
        })
    }

    fn get_score_holder(
        &self,
        fn_name: &ResourceLocation,
        result: String,
        fallback: impl Fn(&usize) -> String,
    ) -> String {
        /// Before Minecraft 1.18 score holder names can't be longer than 40 characters
        const MAX_SCORE_HOLDER_LEN: usize = 40;
        if result.len() <= MAX_SCORE_HOLDER_LEN {
            result
        } else if let Some(id) = self.fn_ids.get(fn_name) {
            fallback(id)
        } else {
            // If this is a missing function, it is ok to use the whole name, even if it is to long.
            // In this case the -ns-_valid score can not be set, but it is not set for missing functions anyways.
            result
        }
    }
}

/// Visible for testing only. This is a binary crate, it is not intended to be used as a library.
pub async fn generate_debug_datapack<'l>(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    config: &GeneratorConfig<'l>,
) -> io::Result<DebugDatapackMetadata> {
    let functions = find_function_files(input_path).await?;
    let fn_ids = functions
        .keys()
        .enumerate()
        .map(|(index, it)| (it.clone(), index))
        .collect::<HashMap<_, _>>();
    let metadata = DebugDatapackMetadata { fn_ids };

    let fn_contents = parse_functions(&functions).await?;

    let output_name = output_path
        .as_ref()
        .file_name()
        .and_then(OsStr::to_str)
        .unwrap_or_default();
    let engine = TemplateEngine::new(
        BTreeMap::from_iter([("-ns-", config.namespace), ("-datapack-", output_name)]),
        config.adapter_listener_name,
    );
    expand_templates(&engine, &metadata, &fn_contents, &output_path).await?;

    write_functions_txt(functions.keys(), &output_path).await?;

    Ok(metadata)
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
) -> Result<HashMap<&'l ResourceLocation, Vec<(usize, String, Line)>>, io::Error> {
    let parser =
        CommandParser::default().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    functions
        .iter()
        .map(|(name, path)| {
            // TODO async
            let lines = read_to_string(path)?
                .split('\n')
                .enumerate()
                .map(|(line_index, line)| {
                    let line = line.strip_suffix('\r').unwrap_or(line); // Remove trailing carriage return on Windows
                    let command = parse_line(&parser, line);
                    (line_index + 1, line.to_string(), command)
                })
                .collect::<Vec<(usize, String, Line)>>();
            Ok((name, lines))
        })
        .collect()
}

async fn expand_templates(
    engine: &TemplateEngine<'_>,
    metadata: &DebugDatapackMetadata,
    fn_contents: &HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
    output_path: impl AsRef<Path>,
) -> io::Result<()> {
    try_join!(
        expand_global_templates(engine, metadata, fn_contents, &output_path),
        expand_function_specific_templates(engine, metadata, fn_contents, &output_path),
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
    metadata: &DebugDatapackMetadata,
    fn_contents: &HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
    output_path: impl AsRef<Path>,
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
        expand!("data/-ns-/functions/prepare_resume.mcfunction"),
        expand!("data/-ns-/functions/reset_skipped.mcfunction"),
        expand_schedule_template(&engine, fn_contents, &output_path),
        expand!("data/-ns-/functions/select_entity.mcfunction"),
        expand!("data/-ns-/functions/skipped_functions_warning.mcfunction"),
        expand!("data/-ns-/functions/tick_start.mcfunction"),
        expand!("data/-ns-/functions/tick.mcfunction"),
        expand!("data/-ns-/functions/unfreeze_aec.mcfunction"),
        expand!("data/-ns-/functions/uninstall.mcfunction"),
        expand_scores_templates(&engine, fn_contents, &output_path),
        expand_validate_all_functions_template(&engine, metadata, fn_contents, &output_path),
        expand!("data/debug/functions/install.mcfunction"),
        expand_show_skipped_template(&engine, metadata, fn_contents, &output_path),
        expand!("data/debug/functions/stop.mcfunction"),
        expand!("data/debug/functions/uninstall.mcfunction"),
        expand!("data/minecraft/tags/functions/load.json"),
        expand!("data/minecraft/tags/functions/tick.json"),
        expand!("pack.mcmeta"),
    )?;

    Ok(())
}

async fn expand_schedule_template(
    engine: &TemplateEngine<'_>,
    fn_contents: &HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
    output_path: impl AsRef<Path>,
) -> io::Result<()> {
    #[rustfmt::skip]
  macro_rules! PATH { () => { "data/-ns-/functions/schedule.mcfunction" }; }

    let content = fn_contents
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
    fn_contents: &HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
    output_path: impl AsRef<Path>,
) -> io::Result<()> {
    let objectives = fn_contents
        .values()
        .flat_map(|vec| vec)
        .filter_map(|(_, _, line)| line.objectives())
        .flat_map(|objectives| objectives)
        .collect::<BTreeSet<_>>();

    expand_log_scores_template(&objectives, engine, &output_path).await?;

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

async fn expand_validate_all_functions_template(
    engine: &TemplateEngine<'_>,
    metadata: &DebugDatapackMetadata,
    fn_contents: &HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
    output_path: impl AsRef<Path>,
) -> io::Result<()> {
    #[rustfmt::skip]
  macro_rules! PATH { () => { "data/-ns-/functions/validate_all_functions.mcfunction" }; }

    let content = fn_contents
        .keys()
        .map(|name| {
            let fn_score_holder = metadata.get_fn_score_holder(name);
            let engine = engine
                .extend_orig_name(name)
                .extend([("-fn_score_holder-", fn_score_holder.as_str())]);
            engine.expand(include_template!(PATH!()))
        })
        .collect::<Vec<_>>()
        .join("");

    let path = output_path.as_ref().join(engine.expand(PATH!()));
    write(&path, &content).await
}

async fn expand_show_skipped_template(
    engine: &TemplateEngine<'_>,
    metadata: &DebugDatapackMetadata,
    fn_contents: &HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
    output_path: impl AsRef<Path>,
) -> io::Result<()> {
    // This may include calls to non-existent functions
    let called_functions = fn_contents
        .values()
        .flat_map(|vec| vec)
        .filter_map(|(_, _, line)| match line {
            Line::FunctionCall { name, .. } => Some(name),
            _ => None,
        })
        .collect::<BTreeSet<_>>();

    let execute_if_skipped = "execute if score -fn_score_holder- -ns-_skipped matches 1..";
    let is_valid = "score -fn_score_holder- -ns-_valid matches 0";
    let tellraw = r#"tellraw @s [{"text":" - -orig_ns-:-orig/fn- ("},{"score":{"name":"-orig_ns-:-orig/fn-","objective":"-ns-_skipped"}},{"text":"x)"}]"#;

    let missing_functions = called_functions
        .iter()
        .map(|orig_name| {
            let fn_score_holder = metadata.get_fn_score_holder(orig_name);
            engine
                .extend_orig_name(orig_name)
                .extend([("-fn_score_holder-", fn_score_holder.as_str())])
                .expand(&format!(
                    "{} {} {} run {}",
                    execute_if_skipped, "unless", is_valid, tellraw
                ))
        })
        .collect::<Vec<_>>()
        .join("\n");

    let invalid_functions = called_functions
        .iter()
        .map(|orig_name| {
            let fn_score_holder = metadata.get_fn_score_holder(orig_name);
            engine
                .extend_orig_name(orig_name)
                .extend([("-fn_score_holder-", fn_score_holder.as_str())])
                .expand(&format!(
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
    metadata: &DebugDatapackMetadata,
    fn_contents: &HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
    output_path: impl AsRef<Path>,
) -> io::Result<()> {
    let call_tree = create_call_tree(&fn_contents);

    try_join_all(fn_contents.iter().map(|(fn_name, lines)| {
        expand_function_templates(&engine, fn_name, lines, metadata, &call_tree, &output_path)
    }))
    .await?;

    Ok(())
}

fn create_call_tree<'l>(
    fn_contents: &'l HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
) -> MultiMap<&'l ResourceLocation, (&'l ResourceLocation, &'l usize)> {
    fn_contents
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
    metadata: &DebugDatapackMetadata,
    call_tree: &MultiMap<&ResourceLocation, (&ResourceLocation, &usize)>,
    output_path: impl AsRef<Path>,
) -> io::Result<()> {
    let fn_score_holder = metadata.get_fn_score_holder(fn_name);
    let engine = engine
        .extend_orig_name(fn_name)
        .extend([("-fn_score_holder-", fn_score_holder.as_str())]);

    let output_path = output_path.as_ref();
    let fn_dir = output_path.join(engine.expand("data/-ns-/functions/-orig_ns-/-orig/fn-"));
    create_dir_all(&fn_dir).await?;

    let partitions = partition(lines);

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

        let terminator = match &partition.terminator {
            Terminator::ConfigurableBreakpoint { position_in_line } => {
                let column = match position_in_line {
                    BreakpointPositionInLine::Breakpoint => 1,
                    BreakpointPositionInLine::AfterFunction => {
                        let (_line_number, line, _parsed) = &lines[partition.end.line_number - 1];
                        1 + line.len()
                    }
                };
                let position = LocalBreakpointPosition {
                    line_number: partition.end.line_number,
                    position_in_line: *position_in_line,
                };
                let next_partition = &partitions[partition_index + 1];
                expand_breakpoint_template(
                    &engine,
                    output_path,
                    &metadata,
                    &fn_name,
                    &position,
                    column,
                    next_partition,
                )
                .await?
            }
            Terminator::FunctionCall {
                column_index,
                line,
                name: called_fn,
                anchor,
                selectors,
            } => {
                let line_number = (partition.end.line_number).to_string();
                let fn_score_holder = metadata.get_fn_score_holder(called_fn);
                let execute = &line[..*column_index];
                let execute = exclude_internal_entites_from_selectors(execute, selectors);
                let debug_anchor = anchor.map_or("".to_string(), |anchor| {
                    let mut anchor_score = 0;
                    if anchor == MinecraftEntityAnchor::EYES {
                        anchor_score = 1;
                    }
                    format!(
                        "execute if score -fn_score_holder- -ns-_valid matches 1 run \
                          scoreboard players set current -ns-_anchor {anchor_score}",
                        anchor_score = anchor_score
                    )
                });
                let engine = engine.extend([
                    ("-line_number-", line_number.as_str()),
                    ("-call_ns-", called_fn.namespace()),
                    ("-call/fn-", called_fn.path()),
                    ("-fn_score_holder-", fn_score_holder.as_str()),
                    ("execute run ", &execute),
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

    if let Some(callers) = call_tree.get_vec(fn_name) {
        let mut return_cases = callers
            .iter()
            .map(|(caller, line_number)| {
                engine.expand(&format!(
                    "execute if entity \
                  @s[tag=-ns-+{caller_ns}+{caller_fn_tag}+{line_number}] run \
                  function -ns-:{caller_ns}/{caller_fn}/\
                  continue_current_iteration_at_{line_number}_function",
                    caller_ns = caller.namespace(),
                    caller_fn = caller.path(),
                    caller_fn_tag = caller.path().replace("/", "+"),
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
            Line::Empty | Line::Comment => line.to_string(),
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

async fn expand_breakpoint_template(
    engine: &TemplateEngine<'_>,
    output_path: &Path,
    metadata: &DebugDatapackMetadata,
    fn_name: &ResourceLocation,
    position: &LocalBreakpointPosition,
    column: usize,
    next_partition: &Partition<'_>,
) -> io::Result<String> {
    let score_holder = metadata.get_breakpoint_score_holder(fn_name, position);

    let line_number = position.line_number.to_string();
    let position = position.to_string();
    let column_str = &format!(":{}", column);
    let optional_column = if column == 0 { "" } else { column_str };
    let engine = engine.extend([
        ("-line_number-", line_number.as_str()),
        ("-position-", &position),
        ("-optional_column-", optional_column),
    ]);
    expand_template!(
        engine,
        output_path,
        "data/-ns-/functions/-orig_ns-/-orig/fn-/suspend_at_-position-.mcfunction"
    )
    .await?;

    let next_positions = format!("{}-{}", next_partition.start, next_partition.end);
    let engine = engine.extend([
        ("-next_positions-", next_positions.as_str()),
        ("-score_holder-", score_holder.as_str()),
    ]);
    Ok(engine.expand(include_template!(
        "data/template/functions/breakpoint_configurable.mcfunction"
    )))
}

async fn write_functions_txt(
    fn_names: impl IntoIterator<Item = &ResourceLocation>,
    output_path: impl AsRef<Path>,
) -> io::Result<()> {
    let path = output_path.as_ref().join("functions.txt");
    let content = fn_names
        .into_iter()
        .map(|it| it.to_string())
        .collect::<Vec<_>>()
        .join("\n");
    write(&path, content).await?;

    Ok(())
}

async fn create_parent_dir(path: impl AsRef<Path>) -> io::Result<()> {
    if let Some(parent_dir) = path.as_ref().parent() {
        create_dir_all(parent_dir).await?;
    }
    Ok(())
}
