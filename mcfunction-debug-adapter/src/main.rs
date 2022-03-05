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

use clap::{crate_authors, crate_version, App};
use debug_adapter_protocol::{
    events::{
        BreakpointEventBody, BreakpointEventReason, Event, StoppedEventBody, StoppedEventReason,
        TerminatedEventBody,
    },
    requests::{
        ContinueRequestArguments, InitializeRequestArguments, LaunchRequestArguments, Request,
        ScopesRequestArguments, SetBreakpointsRequestArguments, StackTraceRequestArguments,
    },
    responses::{
        ContinueResponseBody, ErrorResponse, ErrorResponseBody, ScopesResponseBody,
        SetBreakpointsResponseBody, StackTraceResponseBody, SuccessResponse, ThreadsResponseBody,
    },
    types::{
        Breakpoint, Capabilities, Message, Source, SourcePresentationHint, StackFrame, Thread,
    },
    ProtocolMessage, ProtocolMessageType,
};
use log::info;
use mcfunction_debug_adapter::{read_msg, MessageWriter};
use mcfunction_debugger::{
    generate_debug_datapack, parser::command::resource_location::ResourceLocation,
};
use minect::{log_observer::LogEvent, MinecraftConnection, MinecraftConnectionBuilder};
use simplelog::{Config, WriteLogger};
use std::{
    io,
    path::{Path, PathBuf},
};
use tokio::{
    fs::File,
    io::{AsyncWriteExt, BufReader},
    select,
    sync::mpsc::UnboundedReceiver,
};

const TEST_WORLD_DIR: &str = env!("TEST_WORLD_DIR");
const TEST_LOG_FILE: &str = env!("TEST_LOG_FILE");

#[tokio::main]
async fn main() -> io::Result<()> {
    App::new("mcfunction-debug-adapter")
        .version(crate_version!())
        .long_version(concat!(
            crate_version!(),
            " (Commit: ",
            env!("VERGEN_GIT_SHA"),
            ")"
        ))
        .version_short("v")
        .author(&*format!(
            "
Vanilla Technologies
© Copyright (C) 2021 {}

mcfunction-debugger is free software: you can redistribute it and/or modify it
under the terms of the GNU General Public License as published by the Free
Software Foundation, either version 3 of the License, or (at your option) any
later version.
mcfunction-debugger is distributed in the hope that it will be useful, but
WITHOUT ANY WARRANTY; without even the implied warranty of MERCHANTABILITY or
FITNESS FOR A PARTICULAR PURPOSE.
See the GNU General Public License for more details.

",
            crate_authors!(" & ")
        ))
        .get_matches();

    let project_dir = Path::new(env!("PWD"));
    WriteLogger::init(
        log::LevelFilter::Trace,
        Config::default(),
        std::fs::File::create(project_dir.join("std.log"))?,
    )
    .unwrap();

    match run().await {
        Err(e) => {
            let mut err_log = File::create(project_dir.join("err.log")).await?;
            err_log.write_all(e.to_string().as_bytes()).await?;
            Err(e)
        }
        _ => Ok(()),
    }
}

async fn run() -> io::Result<()> {
    let mut stdin = BufReader::new(tokio::io::stdin());
    let project_dir = Path::new(env!("PWD"));
    let mut in_log = File::create(project_dir.join("in.log")).await?;
    let mut out_log = File::create(project_dir.join("out.log")).await?;
    // let mut writer = MessageWriter::new(tokio::io::stdout(), &mut out_log);

    let mut adapter = DebugAdapter::new(tokio::io::stdout(), &mut out_log);

    loop {
        select! {
            client_msg = read_msg(&mut stdin, &mut in_log) => {
                let client_msg = client_msg?;
                let should_continue = adapter.handle_client_message(client_msg).await?;
                if !should_continue {
                    break;
                }
            },
            Some(minecraft_msg) = adapter.recv_minecraft_msg() => {
                info!("Received message from Minecraft by {}: {}", minecraft_msg.executor, minecraft_msg.message);
                adapter.handle_minecraft_message(minecraft_msg).await?;
            },
        }
    }

    in_log.write("finished".as_bytes()).await?;
    out_log.write("finished".as_bytes()).await?;

    Ok(())
}

fn with_command(command: &'static str) -> impl Fn((String, Option<Message>)) -> ErrorResponse {
    move |(message, error)| ErrorResponse {
        command: command.to_string(),
        message,
        body: ErrorResponseBody { error },
    }
}

fn extract_function_name(data_path: &Path, program: &Path) -> io::Result<ResourceLocation> {
    let namespace = data_path
        .iter()
        .next()
        .ok_or_else(|| {
            invalid_input(format!(
                "Attribute 'program' contains an invalid path: {}",
                program.display()
            ))
        })?
        .to_str()
        .unwrap() // Path is known to be UTF-8
        ;
    let fn_path = data_path
        .strip_prefix(Path::new(namespace).join("functions"))
        .map_err(|_| {
            invalid_input(format!(
                "Attribute 'program' contains an invalid path: {}",
                program.display()
            ))
        })?
        .with_extension("")
        .to_str()
        .unwrap() // Path is known to be UTF-8
        .replace(std::path::MAIN_SEPARATOR, "/");
    Ok(ResourceLocation::new(&namespace, &fn_path))
}

fn find_parent_datapack(mut path: &Path) -> Option<&Path> {
    while let Some(p) = path.parent() {
        path = p;
        let pack_mcmeta_path = path.join("pack.mcmeta");
        if pack_mcmeta_path.is_file() {
            return Some(path);
        }
    }
    None
}

fn invalid_input<E>(e: E) -> io::Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    io::Error::new(io::ErrorKind::InvalidInput, e)
}

const ADAPTER_LISTENER_NAME: &'static str = "mcfunction_debugger";

struct DebugAdapter<O, L>
where
    O: AsyncWriteExt + Unpin,
    L: AsyncWriteExt + Unpin,
{
    writer: MessageWriter<O, L>,
    session: Option<Session>,
}

impl<O, L> DebugAdapter<O, L>
where
    O: AsyncWriteExt + Unpin,
    L: AsyncWriteExt + Unpin,
{
    fn new(output: O, log: L) -> DebugAdapter<O, L> {
        DebugAdapter {
            writer: MessageWriter::new(output, log),
            session: None,
        }
    }

    async fn handle_client_message(&mut self, msg: ProtocolMessage) -> io::Result<bool> {
        match msg.type_ {
            ProtocolMessageType::Request(request) => match request {
                Request::Initialize(args) => {
                    let result = self.initialize(args).await?;

                    self.writer.respond(msg.seq, result).await?;

                    self.writer
                        .write_msg(ProtocolMessageType::Event(Event::Initialized))
                        .await?;
                }
                Request::ConfigurationDone => {
                    self.writer
                        .respond(msg.seq, Ok(SuccessResponse::ConfigurationDone))
                        .await?;
                }
                Request::Launch(args) => {
                    let response = self
                        .launch(args)
                        .await?
                        .map(|()| SuccessResponse::Launch)
                        .map_err(with_command("launch"));

                    self.writer.respond(msg.seq, response).await?;
                }
                Request::SetBreakpoints(SetBreakpointsRequestArguments { breakpoints, .. }) => {
                    let breakpoints = breakpoints
                        .iter()
                        .map(|breakpoint| Breakpoint {
                            id: Some(0),
                            verified: true,
                            message: Some("Hello".to_string()),
                            source: None,
                            line: Some(breakpoint.line + 1),
                            column: None,
                            end_line: None,
                            end_column: None,
                            instruction_reference: None,
                            offset: None,
                        })
                        .collect();
                    self.writer
                        .respond(
                            msg.seq,
                            Ok(SuccessResponse::SetBreakpoints(
                                SetBreakpointsResponseBody { breakpoints },
                            )),
                        )
                        .await?;
                }
                Request::Threads => {
                    self.writer
                        .respond(
                            msg.seq,
                            Ok(SuccessResponse::Threads(ThreadsResponseBody {
                                threads: vec![Thread {
                                    id: 0,
                                    name: "My Thread".to_string(),
                                }],
                            })),
                        )
                        .await?;
                }
                Request::StackTrace(StackTraceRequestArguments {
                    thread_id: _,
                    start_frame: _,
                    levels: _,
                    format: _, // TODO: supportsValueFormattingOptions
                }) => {
                    self.writer
                        .respond(
                            msg.seq,
                            Ok(SuccessResponse::StackTrace(StackTraceResponseBody {
                                stack_frames: vec![StackFrame {
                                    id: 0,
                                    name: "example:main".to_string(),
                                    source: Some(Source {
                                        name: Some("Hi I am a Breakpoint!".to_string()),
                                        path: Some(
                                            self.session
                                                .as_ref()
                                                .unwrap()
                                                .datapack
                                                .join("data/sample/functions/main.mcfunction")
                                                .display()
                                                .to_string(),
                                        ),
                                        source_reference: None,
                                        presentation_hint: None,
                                        origin: None,
                                        sources: Vec::new(),
                                        adapter_data: None,
                                        checksums: Vec::new(),
                                    }),
                                    line: 2,
                                    column: 0,
                                    end_line: None,
                                    end_column: None,
                                    can_restart: None,
                                    instruction_pointer_reference: None,
                                    module_id: None,
                                    presentation_hint: None,
                                }],
                                total_frames: Some(1),
                            })),
                        )
                        .await?;
                }
                Request::Scopes(ScopesRequestArguments { frame_id: _ }) => {
                    self.writer
                        .respond(
                            msg.seq,
                            Ok(SuccessResponse::Scopes(ScopesResponseBody {
                                scopes: Vec::new(),
                            })),
                        )
                        .await?;
                }
                Request::Continue(args) => {
                    let response = self
                        .resume(args)
                        .await?
                        .map(|body| SuccessResponse::Continue(body))
                        .map_err(with_command("continue"));

                    self.writer.respond(msg.seq, response).await?;
                }
                Request::Disconnect(_) => {
                    self.writer
                        .respond(msg.seq, Ok(SuccessResponse::Disconnect))
                        .await?;
                    return Ok(false);
                }
                _ => {}
            },
            _ => {}
        }
        Ok(true)
    }

    async fn recv_minecraft_msg(&mut self) -> Option<LogEvent> {
        let session = self.session.as_mut()?;
        session.listener.recv().await
    }

    async fn handle_minecraft_message(&mut self, msg: LogEvent) -> io::Result<()> {
        if let Some(suffix) = msg.message.strip_prefix("Added tag '") {
            if let Some(tag) = suffix.strip_suffix(&format!("' to {}", ADAPTER_LISTENER_NAME)) {
                if tag == "terminated" {
                    self.writer
                        .write_msg(ProtocolMessageType::Event(Event::Terminated(
                            TerminatedEventBody { restart: None },
                        )))
                        .await?;
                }
                // -ns-+-orig_ns-+-orig_fn-+-line_number-
                if let Some((path, line)) = parse_stopped_tag(tag) {
                    // self.writer
                    //     .write_msg(ProtocolMessageType::Event(Event::Breakpoint(
                    //         BreakpointEventBody {
                    //             reason: BreakpointEventReason::New,
                    //             breakpoint: Breakpoint {
                    //                 id: Some(1),
                    //                 verified: true,
                    //                 message: None,
                    //                 source: Some(Source {
                    //                     name: Some("Hi I am a Breakpoint!".to_string()),
                    //                     path: Some(
                    //                         self.session
                    //                             .as_ref()
                    //                             .unwrap()
                    //                             .datapack
                    //                             .join(path)
                    //                             .display()
                    //                             .to_string(),
                    //                     ),
                    //                     source_reference: None,
                    //                     presentation_hint: None,
                    //                     origin: None,
                    //                     sources: Vec::new(),
                    //                     adapter_data: None,
                    //                     checksums: Vec::new(),
                    //                 }),
                    //                 line: Some(line),
                    //                 column: None,
                    //                 end_line: None,
                    //                 end_column: None,
                    //                 instruction_reference: None,
                    //                 offset: None,
                    //             },
                    //         },
                    //     )))
                    //     .await?;

                    self.writer
                        .write_msg(ProtocolMessageType::Event(Event::Stopped(
                            StoppedEventBody {
                                reason: StoppedEventReason::Breakpoint,
                                description: None,
                                thread_id: Some(0),
                                preserve_focus_hint: false,
                                text: None,
                                all_threads_stopped: false,
                                hit_breakpoint_ids: vec![1],
                            },
                        )))
                        .await?;
                }
            }
        }

        // tag @s add bla
        // [16:47:37] [Server thread/INFO]: [Adrodoc: Added tag 'bla' to Adrodoc]
        // scoreboard players set @s mcfd_Age 5
        // [16:49:23] [Server thread/INFO]: [Adrodoc: Set [mcfd_Age] for Adrodoc to 5]
        // scoreboard players add @s mcfd_Age 0
        // [17:00:40] [Server thread/INFO]: [Adrodoc: Added 0 to [mcfd_Age] for Adrodoc (now 5)]

        // Tags allow: 0-9a-zA-Z._+-

        // [16:47:37] [Server thread/INFO]: [mcfunction_debugger: Added tag 'terminated' to mcfunction_debugger]
        // [16:47:37] [Server thread/INFO]: [mcfunction_debugger: Added tag 'stopped_at_breakpoint.-ns-_-orig_ns-_-orig_fn-_-line_number-' to mcfunction_debugger]
        // [16:47:37] [Server thread/INFO]: [mcfunction_debugger: Added tag 'scores.start' to mcfunction_debugger]
        // [16:49:23] [Server thread/INFO]: [mcfunction_debugger: Added 0 to [mcfd_Age] for Adrodoc (now 5)]
        // [16:47:37] [Server thread/INFO]: [mcfunction_debugger: Added tag 'scores.end' to mcfunction_debugger]

        Ok(())
    }

    async fn initialize(
        &mut self,
        arguments: InitializeRequestArguments,
    ) -> io::Result<Result<SuccessResponse, ErrorResponse>> {
        // TODO Use world from LaunchRequestArguments
        let mut connection = MinecraftConnectionBuilder::from_ref("dap", TEST_WORLD_DIR)
            .log_file(TEST_LOG_FILE.into())
            .build();
        let listener = connection.add_listener(ADAPTER_LISTENER_NAME);
        self.session = Some(Session {
            connection,
            listener,
            datapack: PathBuf::new(), // FIXME: create session in launch
        });

        Ok(Ok(SuccessResponse::Initialize(Capabilities {
            supports_configuration_done_request: true,
            ..Default::default()
        })))
    }

    async fn launch(
        &mut self,
        args: LaunchRequestArguments,
    ) -> io::Result<Result<(), (String, Option<Message>)>> {
        if let Some(session) = &mut self.session {
            session.launch(args).await
        } else {
            Ok(Err(("uninitialized".to_string(), None)))
        }
    }

    async fn resume(
        &mut self,
        args: ContinueRequestArguments,
    ) -> io::Result<Result<ContinueResponseBody, (String, Option<Message>)>> {
        if let Some(session) = &mut self.session {
            session.resume(args).await
        } else {
            Ok(Err(("uninitialized".to_string(), None)))
        }
    }
}

fn parse_stopped_tag(tag: &str) -> Option<(String, i32)> {
    let breakpoint_tag = tag.strip_prefix("stopped_at_breakpoint.")?;

    if let [orig_ns, orig_fn @ .., line_number] =
        breakpoint_tag.split('+').collect::<Vec<_>>().as_slice()
    {
        let path = format!(
            "data/{}/functions/{}.mcfunction",
            orig_ns,
            orig_fn.join("/")
        );
        let line = line_number.parse::<i32>().ok()?;
        Some((path, line))
    } else {
        None
    }
}

struct Session {
    connection: MinecraftConnection,
    listener: UnboundedReceiver<LogEvent>,
    datapack: PathBuf,
}
impl Session {
    async fn launch(
        &mut self,
        args: LaunchRequestArguments,
    ) -> Result<Result<(), (String, Option<Message>)>, io::Error> {
        // FIXME: Proper launch parameters
        // let datapack = args
        //     .additional_attributes
        //     .get("datapack")
        //     .ok_or_else(|| invalid_data("Missing attribute 'datapack'"))?
        //     .as_str()
        //     .ok_or_else(|| invalid_data("Attribute 'datapack' is not of type string"))?;
        let program = args
            .additional_attributes
            .get("program")
            .ok_or_else(|| invalid_input("Missing attribute 'program'"))?
            .as_str()
            .ok_or_else(|| invalid_input("Attribute 'program' is not of type string"))?;
        let program = Path::new(program);

        let datapack = find_parent_datapack(program).ok_or_else(|| {
            invalid_input(
                "Attribute 'program' \
                does not denote a path in a datapack directory with a pack.mcmeta file",
            )
        })?;
        self.datapack = datapack.to_path_buf();

        let data_path = program.strip_prefix(datapack.join("data")).map_err(|_| {
            invalid_input(format!(
                "Attribute 'program' \
        does not denote a path in the data directory of datapack {}",
                datapack.display()
            ))
        })?;

        let function = extract_function_name(data_path, program)?;

        let datapack_name = datapack
            .file_name()
            .ok_or_else(|| {
                invalid_input(format!(
                    "Attribute 'program' contains an invalid path: {}",
                    program.display()
                ))
            })?
            .to_str()
            .unwrap(); // Path is known to be UTF-8

        let minecraft_world_dir = args
            .additional_attributes
            .get("minecraftWorldDir")
            .ok_or_else(|| invalid_input("Missing attribute 'minecraftWorldDir'"))?
            .as_str()
            .ok_or_else(|| invalid_input("Attribute 'minecraftWorldDir' is not of type string"))?;
        let minecraft_world_dir = Path::new(minecraft_world_dir);

        let output_path = minecraft_world_dir
            .join("datapacks")
            .join(format!("debug-{}", datapack_name));
        info!("output_path={}", output_path.display());
        generate_debug_datapack(
            datapack,
            output_path,
            "mcfd",
            false,
            Some(ADAPTER_LISTENER_NAME),
        )
        .await?;

        self.connection.inject_commands(vec![
            // "say injecting command to start debugging".to_string(),
            "reload".to_string(),
            format!(
                "function debug:{}/{}",
                function.namespace(),
                function.path(),
            ),
        ])?;

        Ok(Ok(()))
    }
    async fn resume(
        &mut self,
        args: ContinueRequestArguments,
    ) -> Result<Result<ContinueResponseBody, (String, Option<Message>)>, io::Error> {
        self.connection
            .inject_commands(vec!["function debug:resume".to_string()])?;
        Ok(Ok(ContinueResponseBody {
            all_threads_continued: false,
        }))
    }
}
