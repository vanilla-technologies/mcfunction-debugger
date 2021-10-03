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

#[macro_use]
mod macros;

mod parser;
mod template_engine;

use crate::{
    parser::{parse_line, Line},
    template_engine::TemplateEngine,
};
use futures::{future::try_join_all, FutureExt};
use multimap::MultiMap;
use parser::command::{resource_location::ResourceLocation, CommandParser};
use std::{
    collections::HashMap,
    fs::File,
    io::{self, BufRead},
    iter::{repeat, FromIterator},
    path::{Path, PathBuf},
};
use tokio::{
    fs::{create_dir_all, write},
    task::JoinHandle,
    try_join,
};
use walkdir::WalkDir;

/// Visible for testing only. This is a binary crate, it is not intended to be used as a library.
pub async fn generate_debug_datapack(
    input_path: impl AsRef<Path>,
    output_path: impl AsRef<Path>,
    namespace: &str,
    shadow: bool,
) -> io::Result<()> {
    let parser =
        CommandParser::default().map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
    let functions = find_function_files(input_path).await?;
    let function_contents = functions
        .iter()
        .map(|(name, path)| {
            //TODO async
            let file = File::open(path)?;
            let mut lines = io::BufReader::new(file)
                .lines()
                .enumerate()
                .map(|(line_number, line)| {
                    line.map(|line| {
                        let command = parse_line(&parser, &line);
                        (line_number + 1, line, command)
                    })
                })
                .collect::<io::Result<Vec<(usize, String, Line)>>>()?;

            // TODO dirty hack for when the last line in a file is a function call or breakpoint
            lines.push((
                lines.len() + 1,
                "".to_string(),
                Line::OtherCommand {
                    selectors: Vec::new(),
                },
            ));
            Ok((name, lines))
        })
        .collect::<Result<HashMap<&ResourceLocation, Vec<(usize, String, Line)>>, io::Error>>()?;

    let engine = TemplateEngine::new(HashMap::from_iter([("-ns-", namespace)]));
    expand_templates(&engine, &function_contents, &output_path, shadow).await
}

async fn find_function_files(
    datapack_path: impl AsRef<Path>,
) -> Result<HashMap<ResourceLocation, PathBuf>, io::Error> {
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
        .collect::<HashMap<ResourceLocation, PathBuf>>())
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
                                    &relative_path.with_extension("").display().to_string(),
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

async fn expand_templates(
    engine: &TemplateEngine<'_>,
    function_contents: &HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
    output_path: impl AsRef<Path>,
    shadow: bool,
) -> io::Result<()> {
    try_join!(
        expand_global_templates(engine, function_contents, &output_path),
        expand_function_specific_templates(engine, function_contents, &output_path, shadow),
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
        expand!("data/-ns-/functions/clean_up.mcfunction"),
        expand!("data/-ns-/functions/decrement_age.mcfunction"),
        expand!("data/-ns-/functions/freeze_aec.mcfunction"),
        expand!("data/-ns-/functions/install.mcfunction"),
        expand!("data/-ns-/functions/load.mcfunction"),
        expand!("data/-ns-/functions/resume_immediately.mcfunction"),
        expand_resume_self_template(&engine, function_contents, &output_path),
        expand!("data/-ns-/functions/resume_unchecked.mcfunction"),
        expand_schedule_template(&engine, function_contents, &output_path),
        expand!("data/-ns-/functions/select_entity.mcfunction"),
        expand!("data/-ns-/functions/tick_start.mcfunction"),
        expand!("data/-ns-/functions/tick.mcfunction"),
        expand!("data/-ns-/functions/unfreeze_aec.mcfunction"),
        expand!("data/-ns-/functions/uninstall.mcfunction"),
        expand!("data/debug/functions/install.mcfunction"),
        expand!("data/debug/functions/resume.mcfunction"),
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
) -> io::Result<()> {
    let resume_cases = function_contents
        .iter()
        .flat_map(|(name, lines)| {
            repeat(name).zip(
                lines
                    .iter()
                    .filter(|(_, _, command)| matches!(command, Line::Breakpoint))
                    .map(|it| it.0),
            )
        })
        .map(|(name, line_number)| {
            engine.expand(&format!(
                "execute \
                if entity @s[tag=-ns-_{original_namespace}_{original_function_tag}_{line_number}] \
                run function -ns-:{original_namespace}/{original_function}/\
                {line_number_1}_continue_current_iteration",
                original_namespace = name.namespace(),
                original_function = name.path(),
                original_function_tag = name.path().replace("/", "_"),
                line_number = line_number,
                line_number_1 = line_number + 1
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

async fn expand_function_specific_templates(
    engine: &TemplateEngine<'_>,
    function_contents: &HashMap<&ResourceLocation, Vec<(usize, String, Line)>>,
    output_path: impl AsRef<Path>,
    shadow: bool,
) -> io::Result<()> {
    let call_tree = create_call_tree(&function_contents);

    try_join_all(function_contents.iter().map(|(fn_name, lines)| {
        expand_function_templates(&engine, fn_name, lines, &call_tree, &output_path, shadow)
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
    shadow: bool,
) -> io::Result<()> {
    let orig_fn = fn_name.path();
    let orig_fn_tag = orig_fn.replace('/', "_");
    let engine = engine.extend([
        ("-orig_ns-", fn_name.namespace()),
        ("-orig_fn-", &orig_fn_tag),
        ("-orig/fn-", orig_fn),
    ]);

    let output_path = output_path.as_ref();
    let fn_dir = output_path.join(engine.expand("data/-ns-/functions/-orig_ns-/-orig/fn-"));
    create_dir_all(&fn_dir).await?;

    let mut start_line = 1;
    for partition in lines.split_inclusive(|(_, _, command)| {
        matches!(*command, Line::Breakpoint | Line::FunctionCall { .. })
    }) {
        let first = start_line == 1;
        let end_line = start_line + partition.len();
        let last = end_line == lines.len() + 1;

        let line_number = start_line.to_string();
        let line_numbers = format!("{}-{}", start_line, end_line - 1);
        let engine = engine.extend([
            ("-line_number-", line_number.as_str()),
            ("-line_numbers-", line_numbers.as_str()),
        ]);
        macro_rules! expand {
            ($p:literal) => {
                expand_template!(engine, output_path, $p)
            };
        }

        start_line = end_line;

        if first {
            create_parent_dir(
                output_path.join(engine.expand("data/debug/functions/-orig_ns-/-orig/fn-")),
            )
            .await?;
            let mut futures = vec![
                expand!(
                    "data/-ns-/functions/-orig_ns-/-orig/fn-/next_iteration_or_return.mcfunction"
                ),
                expand!("data/-ns-/functions/-orig_ns-/-orig/fn-/scheduled.mcfunction"),
                expand!("data/-ns-/functions/-orig_ns-/-orig/fn-/start.mcfunction"),
                expand!("data/debug/functions/-orig_ns-/-orig/fn-.mcfunction"),
            ];
            if shadow {
                create_parent_dir(
                    output_path.join(engine.expand("data/-orig_ns-/functions/-orig/fn-")),
                )
                .await?;
                futures.push(expand!("data/-orig_ns-/functions/-orig/fn-.mcfunction"));
            }
            try_join_all(futures).await?;
        } else {
            expand_template!(
                engine,
                output_path,
                "data/-ns-/functions/-orig_ns-/-orig/fn-/-line_number-_continue_current_iteration.mcfunction"
            )
            .await?;
        }

        // -line_number-_continue.mcfunction
        #[rustfmt::skip]
        macro_rules! PATH { () => {"data/-ns-/functions/-orig_ns-/-orig/fn-/-line_number-_continue.mcfunction"} }
        let path = output_path.join(engine.expand(PATH!()));
        let mut template = include_template!(PATH!()).to_string();
        if last {
            template.push_str(include_template!(
                "data/-ns-/functions/-orig_ns-/-orig/fn-/-line_number-_continue_last.mcfunction"
            ));
        }
        write(&path, &engine.expand(&template)).await?;

        // -line_numbers-.mcfunction
        let content = partition
            .iter()
            .map(|line| engine.expand_line(line))
            .collect::<Vec<_>>()
            .join("\n");
        expand_template!(
            engine.extend([("# -content-", content.as_str())]),
            output_path,
            "data/-ns-/functions/-orig_ns-/-orig/fn-/-line_numbers-.mcfunction"
        )
        .await?;
    }

    if let Some(callers) = call_tree.get_vec(fn_name) {
        let return_cases = callers
            .iter()
            .map(|(caller, line_number)| {
                engine.expand(&format!(
                    "execute if entity \
                    @s[tag=-ns-_{caller_namespace}_{caller_function_tag}_{line_number}] run \
                    function -ns-:{caller_namespace}/{caller_function}/{line_number_1}\
                    _continue_current_iteration",
                    caller_namespace = caller.namespace(),
                    caller_function = caller.path(),
                    caller_function_tag = caller.path().replace("/", "_"),
                    line_number = line_number,
                    line_number_1 = *line_number + 1,
                ))
            })
            .collect::<Vec<_>>()
            .join("\n");

        expand_template!(
            engine.extend([("# -return_cases-", return_cases.as_str())]),
            output_path,
            "data/-ns-/functions/-orig_ns-/-orig/fn-/return.mcfunction"
        )
        .await?;
    }

    Ok(())
}

async fn create_parent_dir(path: impl AsRef<Path>) -> io::Result<()> {
    if let Some(parent_dir) = path.as_ref().parent() {
        create_dir_all(parent_dir).await?;
    }
    Ok(())
}
