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

pub mod utils;

use crate::{
    adapter::utils::{
        contains_breakpoint, events_between_tags, generate_datapack, parse_function_path,
        McfunctionBreakpoint, McfunctionBreakpointTag,
    },
    error::{DapError, PartialErrorResponse},
    get_command,
    minecraft::{parse_added_tag_message, parse_scoreboard_value, ScoreboardMessage},
    MessageWriter,
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
        VariablesRequestArguments,
    },
    responses::{
        ContinueResponseBody, EvaluateResponseBody, ScopesResponseBody, SetBreakpointsResponseBody,
        StackTraceResponseBody, SuccessResponse, ThreadsResponseBody, VariablesResponseBody,
    },
    types::{
        Breakpoint, Capabilities, Scope, ScopePresentationHint, Source, StackFrame, Thread,
        Variable,
    },
    ProtocolMessage, ProtocolMessageContent,
};
use futures::{
    stream::{select_all, SelectAll},
    Sink, Stream, StreamExt,
};
use log::{info, trace};
use mcfunction_debugger::{
    parser::{
        command::{resource_location::ResourceLocation, CommandParser},
        parse_line, Line,
    },
    utils::{logged_command, logged_command_str, named_logged_command},
    BreakpointKind, LocalBreakpoint,
};
use minect::{log_observer::LogEvent, MinecraftConnection, MinecraftConnectionBuilder};
use multimap::MultiMap;
use std::{
    convert::TryFrom,
    future::ready,
    io,
    path::{Path, PathBuf},
    pin::Pin,
};
use tokio::{
    fs::File,
    io::{AsyncBufReadExt, BufReader},
};
use tokio_stream::wrappers::{LinesStream, UnboundedReceiverStream};

const LISTENER_NAME: &'static str = "mcfunction_debugger";

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
    generated_breakpoints: MultiMap<ResourceLocation, LocalBreakpoint>,
    stopped_at: Option<McfunctionBreakpoint<String>>,
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
    scopes: Vec<ScopeReference>,
}
impl MinecraftSession {
    fn inject_commands(&mut self, commands: Vec<String>) -> Result<(), PartialErrorResponse> {
        trace!("Injecting commands:\n{}", commands.join("\n"));
        inject_commands(&mut self.connection, commands)
    }

    fn replace_ns(&self, command: &str) -> String {
        command.replace("-ns-", &self.namespace)
    }

    async fn get_context_entity_id(&mut self, depth: i32) -> Result<i32, PartialErrorResponse> {
        let stream = UnboundedReceiverStream::new(self.connection.add_general_listener());

        const START_TAG: &str = "get_context_entity_id.start";
        const END_TAG: &str = "get_context_entity_id.end";

        self.inject_commands(vec![
            logged_command_str("function minect:enable_logging"),
            named_logged_command(LISTENER_NAME, format!("tag @s add {}", START_TAG)),
            logged_command(
                format!(
                    "scoreboard players add @e[\
                        type=area_effect_cloud,\
                        tag=-ns-_context,\
                        tag=-ns-_active,\
                        tag=-ns-_current,\
                        scores={{-ns-_depth={}}},\
                    ] -ns-_id 0",
                    depth
                )
                .replace("-ns-", &self.namespace),
            ),
            named_logged_command(LISTENER_NAME, format!("tag @s add {}", END_TAG)),
            logged_command_str("function minect:reset_logging"),
        ])?;

        events_between_tags(stream, START_TAG, END_TAG)
            .filter_map(|event| {
                ready(parse_scoreboard_value(
                    &event.message,
                    &format!("{}_id", &self.namespace),
                ))
            })
            .next()
            .await
            .ok_or_else(|| PartialErrorResponse::new("Minecraft connection closed".to_string()))
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

const MAIN_THREAD_ID: i32 = 0;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ScopeKind {
    SelectedEntityScores,
}
pub const SELECTED_ENTITY_SCORES: &str = "@s scores";
impl ScopeKind {
    fn get_display_name(&self) -> &'static str {
        match self {
            ScopeKind::SelectedEntityScores => SELECTED_ENTITY_SCORES,
        }
    }
}

struct ScopeReference {
    frame_id: i32,
    kind: ScopeKind,
}

pub struct McfunctionDebugAdapter<O>
where
    O: Sink<ProtocolMessage> + Unpin,
{
    message_streams: SelectAll<Pin<Box<dyn Stream<Item = Message> + Send>>>,
    writer: MessageWriter<O>,
    client_session: Option<ClientSession>,
}
impl<O> McfunctionDebugAdapter<O>
where
    O: Sink<ProtocolMessage, Error = io::Error> + Unpin,
{
    pub fn new<I>(input: I, output: O) -> McfunctionDebugAdapter<O>
    where
        I: Stream<Item = io::Result<ProtocolMessage>> + Unpin + 'static + Send,
    {
        let client_messages: Pin<Box<dyn Stream<Item = Message> + Send>> =
            Box::pin(input.map(Message::Client));
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
                    trace!("Received message from client: {}", client_msg);
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
                    let should_continue = self.handle_minecraft_message(minecraft_msg).await?;
                    if !should_continue {
                        break;
                    }
                }
            }
        }
        trace!("Debug adapter finished");
        Ok(())
    }

    async fn handle_client_message(&mut self, msg: ProtocolMessage) -> io::Result<bool> {
        match msg.content {
            // TODO handle all client requests in handle_client_request
            ProtocolMessageContent::Request(Request::Disconnect(_args)) => {
                self.writer
                    .respond(msg.seq, Ok(SuccessResponse::Disconnect))
                    .await?;
                return Ok(false);
            }
            ProtocolMessageContent::Request(request) => {
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
            Request::Scopes(args) => self.scopes(args).await.map(SuccessResponse::Scopes),
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
            Request::Threads => self.threads().await.map(SuccessResponse::Threads),
            Request::Variables(args) => self.variables(args).await.map(SuccessResponse::Variables),
            _ => {
                let command = get_command(&request);
                Err(DapError::Respond(PartialErrorResponse::new(format!(
                    "Unsupported request {}",
                    command
                ))))
            }
        }
    }

    async fn handle_minecraft_message(&mut self, msg: LogEvent) -> io::Result<bool> {
        if let Some(suffix) = msg.message.strip_prefix("Added tag '") {
            if let Some(tag) = suffix.strip_suffix(&format!("' to {}", LISTENER_NAME)) {
                if tag == "exited" {
                    self.writer
                        .write_msg(TerminatedEventBody::builder().build())
                        .await?;
                    return Ok(false);
                }
                if let Some(tag) = Self::parse_stopped_tag(tag) {
                    self.on_stopped(tag).await?;
                }
            }
        }
        Ok(true)
    }

    fn parse_stopped_tag(tag: &str) -> Option<McfunctionBreakpointTag<String>> {
        let breakpoint_tag = tag.strip_prefix("stopped_at_breakpoint.")?;
        breakpoint_tag.parse().ok()
    }

    async fn on_stopped(&mut self, tag: McfunctionBreakpointTag<String>) -> Result<(), io::Error> {
        self.client_session.as_mut().unwrap().stopped_at = Some(tag.0); // TODO unwrap

        self.writer
            .write_msg(
                StoppedEventBody::builder()
                    .reason(StoppedEventReason::Breakpoint)
                    .thread_id(Some(MAIN_THREAD_ID))
                    .build(),
            )
            .await?;
        Ok(())
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
            generated_breakpoints: MultiMap::new(),
            stopped_at: None,
            parser,
        });

        self.writer
            .write_msg(Event::Initialized)
            .await
            .map_err(|e| DapError::Terminate(e))?;

        Ok(Capabilities::builder()
            .supports_configuration_done_request(true)
            .supports_cancel_request(true)
            .supports_terminate_request(true)
            .build())
    }

    async fn launch(&mut self, args: LaunchRequestArguments) -> Result<(), DapError> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;

        //     self.writer
        //     .write_msg(ProtocolMessageContent::Event(Event::Output(OutputEventBody {
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
        //     .write_msg(ProtocolMessageContent::Event(Event::ProgressStart(
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
        //     .write_msg(ProtocolMessageContent::Event(Event::ProgressEnd(
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
        let listener = connection.add_listener(LISTENER_NAME);
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
            scopes: Vec::new(),
        };

        generate_datapack(
            &minecraft_session,
            &client_session.breakpoints,
            &client_session.generated_breakpoints,
        )
        .await?;

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
            response.push(
                Breakpoint::builder()
                    .id(verified.then(|| id))
                    .verified(verified)
                    .line(Some(line_number as i32 - offset))
                    .build(),
            );
        }

        client_session
            .breakpoints
            .insert_many(function.clone(), new_breakpoints);
        // Unwrap is safe, because we just inserted the value
        let new_breakpoints = client_session.breakpoints.get_vec(&function).unwrap();

        if let Some(minecraft_session) = client_session.minecraft_session.as_mut() {
            generate_datapack(
                minecraft_session,
                &client_session.breakpoints,
                &client_session.generated_breakpoints,
            )
            .await?;
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

        Ok(SetBreakpointsResponseBody::builder()
            .breakpoints(response)
            .build())
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
            logged_command_str("function minect:enable_logging"),
            named_logged_command(LISTENER_NAME, format!("tag @s add {}", START_TAG)),
            logged_command(mc_session.replace_ns(&format!(
                "execute as @e[type=area_effect_cloud,tag=-ns-_function_call] \
                run scoreboard players add @s -ns-_depth 0"
            ))),
            logged_command(mc_session.replace_ns(&format!(
                "execute as @e[type=area_effect_cloud,tag=-ns-_breakpoint] run tag @s add {}",
                stack_trace_tag
            ))),
            logged_command(mc_session.replace_ns(&format!(
                "execute as @e[type=area_effect_cloud,tag=-ns-_breakpoint] run tag @s remove {}",
                stack_trace_tag
            ))),
            named_logged_command(LISTENER_NAME, format!("tag @s add {}", END_TAG)),
            logged_command_str("function minect:reset_logging"),
        ])?;

        let mut stack_trace = events_between_tags(stream, START_TAG, END_TAG)
            .filter_map(|event| ready(Self::parse_stack_frame(event, mc_session, &stack_trace_tag)))
            .map(|stack_frame| (stack_frame.id, stack_frame))
            .collect::<Vec<_>>()
            .await;

        stack_trace.sort_by_key(|it| it.0);

        // TODO: ugly
        let mut stack_trace = stack_trace
            .into_iter()
            .enumerate()
            .map(|(index, (_id, mut stack_frame))| {
                stack_frame.id = index as i32;
                stack_frame
            })
            .collect::<Vec<_>>();

        stack_trace.reverse();

        Ok(StackTraceResponseBody::builder()
            .total_frames(Some(stack_trace.len() as i32))
            .stack_frames(stack_trace)
            .build())
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
        StackFrame::builder()
            .id(id)
            .name(format!("{}:{}", function, line))
            .source(Some(
                Source::builder()
                    .path(Some(
                        datapack
                            .as_ref()
                            .join(&format!(
                                "data/{}/functions/{}.mcfunction",
                                function.namespace(),
                                function.path()
                            ))
                            .display()
                            .to_string(),
                    ))
                    .build(),
            ))
            .line(line)
            .column(0)
            .build()
    }

    async fn continue_(
        &mut self,
        _args: ContinueRequestArguments,
    ) -> Result<ContinueResponseBody, DapError> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let mc_session = Self::unwrap_minecraft_session(&mut client_session.minecraft_session)?;

        if let Some(stopped_at) = client_session.stopped_at.as_ref() {
            // Remove all generated breakpoints with kind continue
            for (_key, values) in client_session.generated_breakpoints.iter_all_mut() {
                values.retain(|it| it.kind != BreakpointKind::Continue);
            }

            client_session.generated_breakpoints.insert(
                stopped_at.function.clone(),
                LocalBreakpoint {
                    line_number: stopped_at.line_number,
                    kind: BreakpointKind::Continue,
                },
            );

            let mut commands = Vec::new();

            if !contains_breakpoint(&client_session.breakpoints, stopped_at) {
                generate_datapack(
                    mc_session,
                    &client_session.breakpoints,
                    &client_session.generated_breakpoints,
                )
                .await?;
                commands.push("reload".to_string());
            };

            commands.push("function debug:resume".to_string());
            mc_session.inject_commands(commands)?;
            client_session.stopped_at = None;
            mc_session.scopes.clear();
        }

        Ok(ContinueResponseBody::builder().build())
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
            .write_msg(
                OutputEventBody::builder()
                    .category(OutputCategory::Important)
                    .output("Minecraft cannot be paused".to_string())
                    .build(),
            )
            .await
            .map_err(|e| DapError::Terminate(e))?;

        Err(DapError::Respond(PartialErrorResponse::new(
            "Minecraft cannot be paused".to_string(),
        )))
    }

    async fn scopes(
        &mut self,
        args: ScopesRequestArguments,
    ) -> Result<ScopesResponseBody, DapError> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let mc_session = Self::unwrap_minecraft_session(&mut client_session.minecraft_session)?;

        let mut scopes = Vec::new();
        let is_server_context = mc_session.get_context_entity_id(args.frame_id).await? == 0;
        if !is_server_context {
            scopes.push(Self::create_selected_entity_scores_scope(mc_session, args));
        }
        Ok(ScopesResponseBody::builder().scopes(scopes).build().into())
    }
    fn create_selected_entity_scores_scope(
        mc_session: &mut MinecraftSession,
        args: ScopesRequestArguments,
    ) -> Scope {
        let kind = ScopeKind::SelectedEntityScores;
        mc_session.scopes.push(ScopeReference {
            frame_id: args.frame_id,
            kind,
        });
        let variables_reference = mc_session.scopes.len();
        Scope::builder()
            .name(kind.get_display_name().to_string())
            .variables_reference(variables_reference as i32)
            .expensive(false)
            .presentation_hint(Some(ScopePresentationHint::Locals)) // TODO: test differences
            .build()
    }

    async fn terminate(&mut self, _args: TerminateRequestArguments) -> Result<(), DapError> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let mc_session = Self::unwrap_minecraft_session(&mut client_session.minecraft_session)?;
        mc_session.inject_commands(vec!["function debug:stop".to_string()])?;
        Ok(())
    }

    async fn threads(&mut self) -> Result<ThreadsResponseBody, DapError> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let _mc_session = Self::unwrap_minecraft_session(&mut client_session.minecraft_session)?;

        let thread = Thread::builder()
            .id(MAIN_THREAD_ID)
            .name("Main Thread".to_string())
            .build();
        Ok(ThreadsResponseBody::builder()
            .threads(vec![thread])
            .build()
            .into())
    }

    async fn variables(
        &mut self,
        args: VariablesRequestArguments,
    ) -> Result<VariablesResponseBody, DapError> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let mc_session = Self::unwrap_minecraft_session(&mut client_session.minecraft_session)?;

        let unknown_variables_reference = || {
            PartialErrorResponse::new(format!(
                "Unknown variables_reference: {}",
                args.variables_reference
            ))
        };
        let scope_id = usize::try_from(args.variables_reference - 1)
            .map_err(|_| unknown_variables_reference())?;
        let scope: &ScopeReference = mc_session
            .scopes
            .get(scope_id)
            .ok_or_else(unknown_variables_reference)?;

        const START_TAG: &str = "variables.start";
        const END_TAG: &str = "variables.end";

        match scope.kind {
            ScopeKind::SelectedEntityScores => {
                let stream =
                    UnboundedReceiverStream::new(mc_session.connection.add_general_listener());

                let execute_as_context = format!(
                    "execute as @e[\
                        type=area_effect_cloud,\
                        tag=-ns-_context,\
                        tag=-ns-_active,\
                        tag=-ns-_current,\
                        scores={{-ns-_depth={}}},\
                    ] run",
                    scope.frame_id
                );
                let decrement_ids = mc_session.replace_ns(&format!(
                    "{} scoreboard players operation @e[tag=!-ns-_context] -ns-_id -= @s -ns-_id",
                    execute_as_context
                ));
                let increment_ids = mc_session.replace_ns(&format!(
                    "{} scoreboard players operation @e[tag=!-ns-_context] -ns-_id += @s -ns-_id",
                    execute_as_context
                ));
                mc_session.inject_commands(vec![
                    logged_command_str("function minect:enable_logging"),
                    named_logged_command(LISTENER_NAME, format!("tag @s add {}", START_TAG)),
                    logged_command(decrement_ids),
                    format!("function -ns-:log_scores").replace("-ns-", &mc_session.namespace),
                    logged_command(increment_ids),
                    named_logged_command(LISTENER_NAME, format!("tag @s add {}", END_TAG)),
                    logged_command_str("function minect:reset_logging"),
                ])?;

                let variables = events_between_tags(stream, START_TAG, END_TAG)
                    .filter_map(|event| ready(ScoreboardMessage::parse(&event.message)))
                    .map(|message| {
                        Variable::builder()
                            .name(message.scoreboard)
                            .value(message.score.to_string())
                            .variables_reference(0)
                            .build()
                    })
                    .collect::<Vec<_>>()
                    .await;

                Ok(VariablesResponseBody::builder()
                    .variables(variables)
                    .build())
            }
        }
    }
}
