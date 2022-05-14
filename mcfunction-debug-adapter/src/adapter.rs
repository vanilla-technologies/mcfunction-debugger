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

use crate::{
    adapter::utils::{
        events_between_tags, generate_datapack, parse_function_path, McfunctionBreakpoint,
        McfunctionBreakpointTag,
    },
    error::{DapError, PartialErrorResponse},
    minecraft::{parse_added_tag_message, parse_scoreboard_value},
};
use debug_adapter_protocol::{
    events::{
        Event, OutputCategory, OutputEventBody, StoppedEventBody, StoppedEventReason,
        TerminatedEventBody,
    },
    requests::{
        ContinueRequestArguments, EvaluateRequestArguments, InitializeRequestArguments,
        LaunchRequestArguments, PathFormat, PauseRequestArguments, Request, ScopesRequestArguments,
        SetBreakpointsRequestArguments, StackTraceRequestArguments, TerminateRequestArguments,
    },
    responses::{
        ContinueResponseBody, EvaluateResponseBody, ScopesResponseBody, SetBreakpointsResponseBody,
        StackTraceResponseBody, SuccessResponse, ThreadsResponseBody,
    },
    types::{Breakpoint, Capabilities, Source, StackFrame, Thread},
    ProtocolMessage, ProtocolMessageType,
};
use futures::{
    stream::{select_all, SelectAll},
    Stream, StreamExt,
};
use log::{info, trace};
use mcfunction_debug_adapter::{get_command, read_msg, MessageWriter};
use mcfunction_debugger::{
    parser::{
        command::{resource_location::ResourceLocation, CommandParser},
        parse_line, Line,
    },
    BreakpointKind, LocalBreakpoint,
};
use minect::{
    log_observer::LogEvent, LoggedCommand, MinecraftConnection, MinecraftConnectionBuilder,
};
use multimap::MultiMap;
use std::{
    future::ready,
    io,
    path::{Path, PathBuf},
    pin::Pin,
};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
};
use tokio_stream::wrappers::{LinesStream, UnboundedReceiverStream};

mod utils;

const ADAPTER_LISTENER_NAME: &'static str = "mcfunction_debugger";

#[derive(Debug)]
enum Message {
    Client(io::Result<ProtocolMessage>),
    Minecraft(LogEvent),
}

struct ClientSession {
    lines_start_at_1: bool,
    path_format: PathFormat,
    minecraft_session: Option<MinecraftSession>,
    breakpoints: MultiMap<ResourceLocation, LocalBreakpoint>,
    parser: CommandParser,
}
impl ClientSession {
    fn get_line_offset(&self) -> i32 {
        if self.lines_start_at_1 {
            0
        } else {
            1
        }
    }
}

struct MinecraftSession {
    connection: MinecraftConnection,
    datapack: PathBuf,
    namespace: String,
    output_path: PathBuf,
}
impl MinecraftSession {
    fn inject_commands(&mut self, commands: Vec<String>) -> Result<(), PartialErrorResponse> {
        trace!("Injecting commands:\n{}", commands.join("\n"));
        inject_commands(&mut self.connection, commands)
    }
}

fn inject_commands(
    connection: &mut MinecraftConnection,
    commands: Vec<String>,
) -> Result<(), PartialErrorResponse> {
    connection
        .inject_commands(commands)
        .map_err(|e| PartialErrorResponse::new(format!("Failed to inject commands: {}", e)))?;
    Ok(())
}

pub struct McfunctionDebugAdapter<O>
where
    O: AsyncWriteExt + Unpin,
{
    message_streams: SelectAll<Pin<Box<dyn Stream<Item = Message>>>>,
    writer: MessageWriter<O>,
    client_session: Option<ClientSession>,
}
impl<O> McfunctionDebugAdapter<O>
where
    O: AsyncWriteExt + Unpin,
{
    pub fn new<I>(mut input: I, output: O) -> McfunctionDebugAdapter<O>
    where
        I: AsyncBufReadExt + Unpin + 'static,
    {
        let client_messages: Pin<Box<dyn Stream<Item = Message>>> =
            Box::pin(async_stream::stream! {
                loop { yield Message::Client(read_msg(&mut input).await); }
            });
        McfunctionDebugAdapter {
            message_streams: select_all([client_messages]),
            writer: MessageWriter::new(output),
            client_session: None,
        }
    }

    pub async fn run(&mut self) -> io::Result<()> {
        trace!("Starting debug adapter");
        while let Some(msg) = self.message_streams.next().await {
            match msg {
                Message::Client(client_msg) => {
                    let client_msg = client_msg?;
                    let should_continue = self.handle_client_message(client_msg).await?;
                    if !should_continue {
                        break;
                    }
                }
                Message::Minecraft(minecraft_msg) => {
                    trace!(
                        "Received message from Minecraft by {}: {}",
                        minecraft_msg.executor,
                        minecraft_msg.message
                    );
                    self.handle_minecraft_message(minecraft_msg).await?;
                }
            }
        }
        trace!("Debug adapter finished");
        Ok(())
    }

    async fn handle_client_message(&mut self, msg: ProtocolMessage) -> io::Result<bool> {
        match msg.type_ {
            // TODO handle all client requests in handle_client_request
            ProtocolMessageType::Request(Request::Disconnect(_args)) => {
                self.writer
                    .respond(msg.seq, Ok(SuccessResponse::Disconnect))
                    .await?;
                return Ok(false);
            }
            ProtocolMessageType::Request(request) => {
                let command = get_command(&request);
                let result = self.handle_client_request(request).await;

                let response = match result {
                    Ok(response) => Ok(response),
                    Err(DapError::Respond(response)) => Err(response.with_command(command)),
                    Err(DapError::Terminate(e)) => return Err(e),
                };
                self.writer.respond(msg.seq, response).await?;
            }
            _ => {
                todo!("Only requests and RunInTerminalResponse should be sent by the client");
            }
        };

        Ok(true)
    }

    async fn handle_client_request(
        &mut self,
        request: Request,
    ) -> Result<SuccessResponse, DapError> {
        match request {
            Request::ConfigurationDone => Ok(SuccessResponse::ConfigurationDone),
            Request::Continue(args) => self.continue_(args).await.map(SuccessResponse::Continue),
            Request::Evaluate(args) => self.evaluate(args).await.map(SuccessResponse::Evaluate),
            Request::Initialize(args) => {
                self.initialize(args).await.map(SuccessResponse::Initialize)
            }
            Request::Launch(args) => self.launch(args).await.map(|()| SuccessResponse::Launch),
            Request::Pause(args) => self.pause(args).await.map(|()| SuccessResponse::Pause),
            Request::Scopes(ScopesRequestArguments { frame_id: _ }) => {
                Ok(SuccessResponse::Scopes(ScopesResponseBody {
                    scopes: Vec::new(),
                }))
            }
            Request::SetBreakpoints(args) => self
                .set_breakpoints(args)
                .await
                .map(SuccessResponse::SetBreakpoints),
            Request::StackTrace(args) => self
                .stack_trace(args)
                .await
                .map(SuccessResponse::StackTrace),
            Request::Terminate(args) => self
                .terminate(args)
                .await
                .map(|()| SuccessResponse::Terminate),
            Request::Threads => Ok(SuccessResponse::Threads(ThreadsResponseBody {
                threads: vec![Thread {
                    id: 0,
                    name: "My Thread".to_string(),
                }],
            })),
            _ => {
                let command = get_command(&request);
                Err(DapError::Respond(PartialErrorResponse::new(format!(
                    "Unsupported request {}",
                    command
                ))))
            }
        }
    }

    async fn handle_minecraft_message(&mut self, msg: LogEvent) -> io::Result<()> {
        if let Some(suffix) = msg.message.strip_prefix("Added tag '") {
            if let Some(tag) = suffix.strip_suffix(&format!("' to {}", ADAPTER_LISTENER_NAME)) {
                if tag == "exited" {
                    self.writer
                        .write_msg(ProtocolMessageType::Event(Event::Terminated(
                            TerminatedEventBody { restart: None },
                        )))
                        .await?;
                }
                if let Some(_) = Self::parse_stopped_tag(tag) {
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
        Ok(())
    }
    fn parse_stopped_tag(tag: &str) -> Option<McfunctionBreakpointTag<String>> {
        let breakpoint_tag = tag.strip_prefix("stopped_at_breakpoint.")?;
        breakpoint_tag.parse().ok()
    }

    fn unwrap_client_session(
        client_session: &mut Option<ClientSession>,
    ) -> Result<&mut ClientSession, PartialErrorResponse> {
        client_session.as_mut().ok_or_else(|| PartialErrorResponse {
            message: "Not initialized".to_string(),
            details: None,
        })
    }

    fn unwrap_minecraft_session(
        minecraft_session: &mut Option<MinecraftSession>,
    ) -> Result<&mut MinecraftSession, PartialErrorResponse> {
        minecraft_session
            .as_mut()
            .ok_or_else(|| PartialErrorResponse {
                message: "Not launched or attached".to_string(),
                details: None,
            })
    }

    async fn initialize(
        &mut self,
        arguments: InitializeRequestArguments,
    ) -> Result<Capabilities, DapError> {
        let parser = CommandParser::default()
            .map_err(|e| DapError::Terminate(io::Error::new(io::ErrorKind::InvalidData, e)))?;
        self.client_session = Some(ClientSession {
            lines_start_at_1: arguments.lines_start_at_1,
            path_format: arguments.path_format,
            minecraft_session: None,
            breakpoints: MultiMap::new(),
            parser,
        });

        self.writer
            .write_msg(ProtocolMessageType::Event(Event::Initialized))
            .await
            .map_err(|e| DapError::Terminate(e))?;

        Ok(Capabilities {
            supports_configuration_done_request: true,
            supports_cancel_request: true,
            supports_terminate_request: true,
            ..Default::default()
        })
    }

    async fn launch(&mut self, args: LaunchRequestArguments) -> Result<(), DapError> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;

        //     self.writer
        //     .write_msg(ProtocolMessageType::Event(Event::Output(OutputEventBody {
        //         category: OutputCategory::Important,
        //         output: "Run /reload in Minecraft".to_string(),
        //         group: None,
        //         variables_reference: None,
        //         source: None,
        //         line: None,
        //         column: None,
        //         data: None,
        //     })))
        //     .await?;

        // let progress_id = Uuid::new_v4();
        // self.writer
        //     .write_msg(ProtocolMessageType::Event(Event::ProgressStart(
        //         ProgressStartEventBody {
        //             progress_id: progress_id.to_string(),
        //             title: "Waiting for connection to Minecraft".to_string(),
        //             request_id: None,
        //             cancellable: true,
        //             message: None,
        //             percentage: None,
        //         },
        //     )))
        //     .await?;

        // sleep(Duration::from_secs(20)).await;

        // self.writer
        //     .write_msg(ProtocolMessageType::Event(Event::ProgressEnd(
        //         ProgressEndEventBody {
        //             progress_id: progress_id.to_string(),
        //             message: Some(
        //                 "Successfully established connection to Minecraft".to_string(),
        //             ),
        //         },
        //     )))
        //     .await?;

        // FIXME: Proper launch parameters
        // let datapack = args
        //     .additional_attributes
        //     .get("datapack")
        //     .ok_or_else(|| invalid_data("Missing attribute 'datapack'"))?
        //     .as_str()
        //     .ok_or_else(|| invalid_data("Attribute 'datapack' is not of type string"))?;

        let program = Self::get_path(&args, "program")?;

        let (datapack, function) = parse_function_path(program)
            .map_err(|e| PartialErrorResponse::new(format!("Attribute 'program' {}", e)))?;

        let datapack_name = datapack
            .file_name()
            .ok_or_else(|| {
                PartialErrorResponse::new(format!(
                    "Attribute 'program' contains an invalid path: {}",
                    program.display()
                ))
            })?
            .to_str()
            .unwrap(); // Path is known to be UTF-8

        let minecraft_world_dir = Self::get_path(&args, "minecraftWorldDir")?;
        let minecraft_log_file = Self::get_path(&args, "minecraftLogFile")?;

        // if connection in filesystem exists {
        // ping
        // timeout -> ?
        // } else {
        // install procedure
        // }

        let mut connection = MinecraftConnectionBuilder::from_ref("dap", minecraft_world_dir)
            .log_file(minecraft_log_file.into())
            .build();
        let listener = connection.add_listener(ADAPTER_LISTENER_NAME);
        let stream = UnboundedReceiverStream::new(listener).map(Message::Minecraft);
        self.message_streams.push(Box::pin(stream));

        let namespace = "mcfd".to_string();
        let output_path = minecraft_world_dir
            .join("datapacks")
            .join(format!("debug-{}", datapack_name));
        info!("output_path={}", output_path.display());

        let mut minecraft_session = MinecraftSession {
            connection,
            datapack,
            namespace,
            output_path,
        };

        generate_datapack(&minecraft_session, &client_session.breakpoints).await?;

        // Install procedure
        // create_installer_datapack

        // connection.inject_commands(vec![logged(
        //     "scoreboard players set minect_reject minect2_global 0",
        // )]);

        // let score = listener.recv().await?;
        // // delete_installer_datapack
        // // delete connection from disk
        // if score == 1 {
        //     return Err("User rejected");
        // }

        inject_commands(
            &mut minecraft_session.connection,
            vec![
                // "say injecting command to start debugging".to_string(),
                "reload".to_string(),
                format!(
                    "function debug:{}/{}",
                    function.namespace(),
                    function.path(),
                ),
            ],
        )?;

        client_session.minecraft_session = Some(minecraft_session);

        Ok(())
    }
    fn get_path<'a>(
        args: &'a LaunchRequestArguments,
        key: &str,
    ) -> Result<&'a Path, PartialErrorResponse> {
        let value = args
            .additional_attributes
            .get(key)
            .ok_or_else(|| PartialErrorResponse::new(format!("Missing attribute '{}'", key)))?
            .as_str()
            .ok_or_else(|| {
                PartialErrorResponse::new(format!("Attribute '{}' is not of type string", key))
            })?;
        let value = Path::new(value);
        Ok(value)
    }

    async fn set_breakpoints(
        &mut self,
        args: SetBreakpointsRequestArguments,
    ) -> Result<SetBreakpointsResponseBody, DapError> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;

        let offset = client_session.get_line_offset();
        let path = match client_session.path_format {
            PathFormat::Path => args.source.path.as_ref().ok_or_else(|| {
                PartialErrorResponse::new("Missing argument source.path".to_string())
            })?,
            PathFormat::URI => todo!("Implement path URIs"),
        };
        let (_datapack, function) = parse_function_path(path)
            .map_err(|e| PartialErrorResponse::new(format!("Argument source.path {}", e)))?;

        let breakpoints = args
            .breakpoints
            .iter()
            .map(|source_breakpoint| (function.clone(), (source_breakpoint.line + offset) as usize))
            .collect::<Vec<_>>();

        let mut response = Vec::new();
        let old_breakpoints = client_session
            .breakpoints
            .remove(&function)
            .unwrap_or_default();
        let mut new_breakpoints = Vec::with_capacity(breakpoints.len());
        for (i, (function, line_number)) in breakpoints.into_iter().enumerate() {
            let id = (i + client_session.breakpoints.len()) as i32;
            let verified = Self::verify_breakpoint(&client_session.parser, path, line_number)
                .await
                .map_err(|e| {
                    PartialErrorResponse::new(format!(
                        "Failed to validate breakpoint {}:{}: {}",
                        function, line_number, e
                    ))
                })?;
            new_breakpoints.push(LocalBreakpoint {
                line_number,
                kind: if verified {
                    BreakpointKind::Normal
                } else {
                    BreakpointKind::Invalid
                },
            });
            response.push(Breakpoint {
                id: verified.then(|| id),
                verified,
                message: None,
                source: None,
                line: Some(line_number as i32 - offset),
                column: None,
                end_line: None,
                end_column: None,
                instruction_reference: None,
                offset: None,
            });
        }

        client_session
            .breakpoints
            .insert_many(function.clone(), new_breakpoints);
        // Unwrap is safe, because we just inserted the value
        let new_breakpoints = client_session.breakpoints.get_vec(&function).unwrap();

        if let Some(minecraft_session) = client_session.minecraft_session.as_mut() {
            generate_datapack(minecraft_session, &client_session.breakpoints).await?;
            let mut commands = vec!["reload".to_string()];
            if args.source_modified && old_breakpoints.len() == new_breakpoints.len() {
                commands.extend(Self::get_move_breakpoint_commands(
                    &function,
                    old_breakpoints.iter().map(|it| it.line_number),
                    new_breakpoints.iter().map(|it| it.line_number),
                    &minecraft_session.namespace,
                ));
            }
            minecraft_session.inject_commands(commands)?;
        }

        Ok(SetBreakpointsResponseBody {
            breakpoints: response,
        })
    }
    async fn verify_breakpoint(
        parser: &CommandParser,
        path: impl AsRef<Path>,
        line_number: usize,
    ) -> io::Result<bool> {
        let file = File::open(path).await?;
        let lines = BufReader::new(file).lines();
        if let Some(result) = LinesStream::new(lines).skip(line_number - 1).next().await {
            let line = result?;
            let line = parse_line(parser, &line, false);
            return Ok(!matches!(line, Line::Empty | Line::Comment));
        } else {
            Ok(false)
        }
    }
    fn get_move_breakpoint_commands(
        function: &ResourceLocation,
        old_line_numbers: impl ExactSizeIterator<Item = usize>,
        new_line_numbers: impl ExactSizeIterator<Item = usize>,
        namespace: &str,
    ) -> Vec<String> {
        let tmp_tag = format!("{}_tmp", namespace);
        let breakpoint_tag = format!("{}_breakpoint", namespace);
        let mut commands = Vec::new();
        for (old_line_number, new_line_number) in old_line_numbers.zip(new_line_numbers) {
            if old_line_number != new_line_number {
                let old_tag = McfunctionBreakpointTag(McfunctionBreakpoint {
                    function: function.to_ref(),
                    line_number: old_line_number,
                });
                let new_tag = McfunctionBreakpointTag(McfunctionBreakpoint {
                    function: function.to_ref(),
                    line_number: new_line_number,
                });
                let old_tag = format!("{}+{}", namespace, old_tag);
                let new_tag = format!("{}+{}", namespace, new_tag);
                commands.push(format!(
                    "tag @e[tag={},tag={},tag=!{}] add {}",
                    breakpoint_tag, old_tag, tmp_tag, new_tag,
                ));
                commands.push(format!(
                    "tag @e[tag={},tag={}] add {}",
                    breakpoint_tag, old_tag, tmp_tag
                ));
                commands.push(format!(
                    "tag @e[tag={},tag={},tag={}] remove {}",
                    breakpoint_tag, old_tag, new_tag, old_tag
                ));
            }
        }
        commands.push(format!(
            "tag @e[tag={},tag={}] remove {}",
            breakpoint_tag, tmp_tag, tmp_tag
        ));
        commands
    }

    async fn stack_trace(
        &mut self,
        _args: StackTraceRequestArguments,
    ) -> Result<StackTraceResponseBody, DapError> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let mc_session = Self::unwrap_minecraft_session(&mut client_session.minecraft_session)?;

        const START_TAG: &str = "stack_trace.start";
        const END_TAG: &str = "stack_trace.end";
        let stack_trace_tag = format!("{}_stack_trace", mc_session.namespace);

        let stream = UnboundedReceiverStream::new(mc_session.connection.add_general_listener());

        mc_session.inject_commands(vec![
            LoggedCommand::from_str("function minect:enable_logging").to_string(),
            LoggedCommand::builder(format!("tag @s add {}", START_TAG))
                .name(ADAPTER_LISTENER_NAME)
                .build()
                .to_string(),
            LoggedCommand::from(format!(
                "execute as @e[type=area_effect_cloud,tag={0}_function_call] \
                run scoreboard players add @s {0}_depth 0",
                mc_session.namespace
            ))
            .to_string(),
            LoggedCommand::from(format!(
                "execute as @e[type=area_effect_cloud,tag={}_breakpoint] run tag @s add {}",
                mc_session.namespace, stack_trace_tag
            ))
            .to_string(),
            LoggedCommand::from(format!(
                "execute as @e[type=area_effect_cloud,tag={}_breakpoint] run tag @s remove {}",
                mc_session.namespace, stack_trace_tag
            ))
            .to_string(),
            LoggedCommand::builder(format!("tag @s add {}", END_TAG))
                .name(ADAPTER_LISTENER_NAME)
                .build()
                .to_string(),
            LoggedCommand::from_str("function minect:reset_logging").to_string(),
        ])?;

        let mut stack_trace = events_between_tags(stream, START_TAG, END_TAG)
            .filter_map(|event| ready(Self::parse_stack_frame(event, mc_session, &stack_trace_tag)))
            .map(|stack_frame| (stack_frame.id, stack_frame))
            .collect::<Vec<_>>()
            .await;

        stack_trace.sort_by_key(|it| -it.0);

        Ok(StackTraceResponseBody {
            total_frames: Some(stack_trace.len() as i32),
            stack_frames: stack_trace.into_iter().map(|it| it.1).collect(),
        })
    }
    fn parse_stack_frame(
        event: LogEvent,
        mc_session: &MinecraftSession,
        stack_trace_tag: &str,
    ) -> Option<StackFrame> {
        if let [orig_ns, orig_fn, line_number] =
            event.executor.split(':').collect::<Vec<_>>().as_slice()
        {
            let line_number = line_number.parse().ok()?;
            let scoreboard = format!("{}_depth", mc_session.namespace);
            let id = if let Some(depth) = parse_scoreboard_value(&event.message, &scoreboard) {
                depth // Function call
            } else if parse_added_tag_message(&event.message)? == stack_trace_tag {
                i32::MAX // Breakpoint
            } else {
                return None;
            };
            let function = ResourceLocation::new(orig_ns, orig_fn);
            let datapack = &mc_session.datapack;
            return Some(Self::new_stack_frame(id, function, line_number, datapack));
        }
        None
    }
    fn new_stack_frame(
        id: i32,
        function: ResourceLocation,
        line: i32,
        datapack: impl AsRef<Path>,
    ) -> StackFrame {
        StackFrame {
            id,
            name: format!("{}:{}", function, line),
            source: Some(Source {
                name: None,
                path: Some(
                    datapack
                        .as_ref()
                        .join(&format!(
                            "data/{}/functions/{}.mcfunction",
                            function.namespace(),
                            function.path()
                        ))
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
            line,
            column: 0,
            end_line: None,
            end_column: None,
            can_restart: None,
            instruction_pointer_reference: None,
            module_id: None,
            presentation_hint: None,
        }
    }

    async fn continue_(
        &mut self,
        _args: ContinueRequestArguments,
    ) -> Result<ContinueResponseBody, DapError> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let mc_session = Self::unwrap_minecraft_session(&mut client_session.minecraft_session)?;

        mc_session.inject_commands(vec!["function debug:resume".to_string()])?;

        Ok(ContinueResponseBody {
            all_threads_continued: false,
        })
    }

    async fn evaluate(
        &mut self,
        _args: EvaluateRequestArguments,
    ) -> Result<EvaluateResponseBody, DapError> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let _mc_session = Self::unwrap_minecraft_session(&mut client_session.minecraft_session)?;

        Err(DapError::Respond(PartialErrorResponse::new(
            "Not supported yet, see: \
            https://github.com/vanilla-technologies/mcfunction-debugger/issues/68"
                .to_string(),
        )))
    }

    async fn pause(&mut self, _args: PauseRequestArguments) -> Result<(), DapError> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let _mc_session = Self::unwrap_minecraft_session(&mut client_session.minecraft_session)?;

        self.writer
            .write_msg(ProtocolMessageType::Event(Event::Output(OutputEventBody {
                category: OutputCategory::Important,
                output: "Minecraft cannot be paused".to_string(),
                group: None,
                variables_reference: None,
                source: None,
                line: None,
                column: None,
                data: None,
            })))
            .await
            .map_err(|e| DapError::Terminate(e))?;

        Err(DapError::Respond(PartialErrorResponse::new(
            "Minecraft cannot be paused".to_string(),
        )))
    }

    async fn terminate(&mut self, _args: TerminateRequestArguments) -> Result<(), DapError> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let mc_session = Self::unwrap_minecraft_session(&mut client_session.minecraft_session)?;
        mc_session.inject_commands(vec!["function debug:stop".to_string()])?;
        Ok(())
    }
}
