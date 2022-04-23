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
        Event, OutputCategory, OutputEventBody, StoppedEventBody, StoppedEventReason,
        TerminatedEventBody,
    },
    requests::{
        ContinueRequestArguments, EvaluateRequestArguments, InitializeRequestArguments,
        LaunchRequestArguments, PauseRequestArguments, Request, ScopesRequestArguments,
        SetBreakpointsRequestArguments, StackTraceRequestArguments, TerminateRequestArguments,
    },
    responses::{
        ContinueResponseBody, ErrorResponse, ErrorResponseBody, EvaluateResponseBody,
        ScopesResponseBody, SetBreakpointsResponseBody, StackTraceResponseBody, SuccessResponse,
        ThreadsResponseBody,
    },
    types::{Breakpoint, Capabilities, Message as ErrorMessage, Source, StackFrame, Thread},
    ProtocolMessage, ProtocolMessageType,
};
use futures::{
    stream::{select_all, SelectAll},
    Stream, StreamExt,
};
use log::{error, info, trace};
use mcfunction_debug_adapter::{read_msg, MessageWriter};
use mcfunction_debugger::{
    generate_debug_datapack, parser::command::resource_location::ResourceLocation,
};
use minect::{
    log_observer::LogEvent, LoggedCommand, MinecraftConnection, MinecraftConnectionBuilder,
};
use serde_json::Value;
use simplelog::{Config, WriteLogger};
use std::{
    io,
    path::{Path, PathBuf},
    pin::Pin,
};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio_stream::wrappers::UnboundedReceiverStream;

#[tokio::main]
async fn main() -> io::Result<()> {
    log_panics::init();

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

    let mut adapter = DebugAdapter::new(BufReader::new(tokio::io::stdin()), tokio::io::stdout());
    match adapter.run().await {
        Err(e) => {
            error!("Stopping due to: {}", e);
            Err(e)
        }
        _ => Ok(()),
    }
}

enum DapError {
    Terminate(io::Error),
    Respond(PartialErrorResponse),
}

impl From<PartialErrorResponse> for DapError {
    fn from(error: PartialErrorResponse) -> Self {
        Self::Respond(error)
    }
}

struct PartialErrorResponse {
    message: String,
    details: Option<ErrorMessage>,
}

impl PartialErrorResponse {
    fn new(message: String) -> PartialErrorResponse {
        PartialErrorResponse {
            message,
            details: None,
        }
    }

    fn with_command(self, command: String) -> ErrorResponse {
        ErrorResponse {
            command,
            message: self.message,
            body: ErrorResponseBody {
                error: self.details,
            },
        }
    }
}

impl From<io::Error> for PartialErrorResponse {
    fn from(error: io::Error) -> Self {
        Self {
            message: error.to_string(),
            details: None,
        }
    }
}

const ADAPTER_LISTENER_NAME: &'static str = "mcfunction_debugger";

#[derive(Debug)]
enum Message {
    Client(io::Result<ProtocolMessage>),
    Minecraft(LogEvent),
}

struct DebugAdapter<O>
where
    O: AsyncWriteExt + Unpin,
{
    message_streams: SelectAll<Pin<Box<dyn Stream<Item = Message>>>>,
    writer: MessageWriter<O>,
    client_session: Option<ClientSession>,
}

impl<O> DebugAdapter<O>
where
    O: AsyncWriteExt + Unpin,
{
    fn new<I>(mut input: I, output: O) -> DebugAdapter<O>
    where
        I: AsyncBufReadExt + Unpin + 'static,
    {
        let client_messages: Pin<Box<dyn Stream<Item = Message>>> =
            Box::pin(async_stream::stream! {
                loop { yield Message::Client(read_msg(&mut input).await); }
            });
        DebugAdapter {
            message_streams: select_all([client_messages]),
            writer: MessageWriter::new(output),
            client_session: None,
        }
    }

    async fn run(&mut self) -> io::Result<()> {
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
                    info!(
                        "Received message from Minecraft by {}: {}",
                        minecraft_msg.executor, minecraft_msg.message
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
                Ok(SuccessResponse::SetBreakpoints(
                    SetBreakpointsResponseBody { breakpoints },
                ))
            }
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
                if let Some(_) = parse_stopped_tag(tag) {
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
        _arguments: InitializeRequestArguments,
    ) -> Result<Capabilities, DapError> {
        self.client_session = Some(ClientSession {
            minecraft_session: None,
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

        let datapack = Self::find_parent_datapack(program).ok_or_else(|| {
            PartialErrorResponse::new(format!(
                "Attribute 'program' \
                does not denote a path in a datapack directory with a pack.mcmeta file: {}",
                program.display()
            ))
        })?;
        let datapack = datapack.to_path_buf();

        let data_path = program.strip_prefix(datapack.join("data")).unwrap();

        let function = Self::get_function_name(data_path, program)?;

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

        let output_path = minecraft_world_dir
            .join("datapacks")
            .join(format!("debug-{}", datapack_name));
        info!("output_path={}", output_path.display());

        let namespace = "mcfd".to_string();

        generate_debug_datapack(
            &datapack,
            output_path,
            &namespace,
            false,
            Some(ADAPTER_LISTENER_NAME),
        )
        .await
        .map_err(|e| {
            PartialErrorResponse::new(format!("Failed to generate debug datapack: {}", e))
        })?;

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
            &mut connection,
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

        client_session.minecraft_session = Some(MinecraftSession {
            connection,
            datapack,
            namespace,
        });

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

    fn get_function_name(
        data_path: &Path,
        program: &Path,
    ) -> Result<ResourceLocation, PartialErrorResponse> {
        let namespace = data_path
            .iter()
            .next()
            .ok_or_else(|| {
                PartialErrorResponse::new(format!(
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
                PartialErrorResponse::new(format!(
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

    async fn stack_trace(
        &mut self,
        _args: StackTraceRequestArguments,
    ) -> Result<StackTraceResponseBody, DapError> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let mc_session = Self::unwrap_minecraft_session(&mut client_session.minecraft_session)?;

        let mut listener = mc_session.connection.add_general_listener();

        let stack_trace_tag = format!("{}_stack_trace", mc_session.namespace);
        const STACK_TRACE_START_TAG: &str = "stack_trace.start";
        const STACK_TRACE_END_TAG: &str = "stack_trace.end";

        mc_session.inject_commands(vec![
            LoggedCommand::from_str("function minect:enable_logging").to_string(),
            LoggedCommand::builder(format!("tag @s add {}", STACK_TRACE_START_TAG))
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
            LoggedCommand::builder(format!("tag @s add {}", STACK_TRACE_END_TAG))
                .name(ADAPTER_LISTENER_NAME)
                .build()
                .to_string(),
            LoggedCommand::from_str("function minect:reset_logging").to_string(),
        ])?;

        trace!(
            "Waiting for tag '{}' on {}",
            STACK_TRACE_START_TAG,
            ADAPTER_LISTENER_NAME
        );
        loop {
            let event = listener.recv().await.unwrap(); // TODO unwrap
            trace!("Got message: {}", event.message);
            if event.executor == ADAPTER_LISTENER_NAME
                && event.message
                    == format!(
                        "Added tag '{1}' to {0}",
                        ADAPTER_LISTENER_NAME, STACK_TRACE_START_TAG
                    )
            {
                break;
            }
        }

        let mut stack_trace = Vec::new();

        trace!(
            "Waiting for tag '{}' on {}",
            STACK_TRACE_END_TAG,
            ADAPTER_LISTENER_NAME
        );
        loop {
            let event = listener.recv().await.unwrap(); // TODO unwrap
            if event.executor == ADAPTER_LISTENER_NAME
                && event.message
                    == format!(
                        "Added tag '{1}' to {0}",
                        ADAPTER_LISTENER_NAME, STACK_TRACE_END_TAG
                    )
            {
                break;
            }

            trace!("Got message: {}", event.message);

            if let [orig_ns, orig_fn, line_number] =
                event.executor.split(':').collect::<Vec<_>>().as_slice()
            {
                if let Some(line) = line_number.parse().ok() {
                    if let Some(depth) =
                        parse_scoreboard_value(&event, &(format!("{}_depth", mc_session.namespace)))
                    {
                        stack_trace.push((
                            depth,
                            Self::new_stack_frame(
                                depth,
                                orig_ns,
                                orig_fn,
                                line,
                                &mc_session.datapack,
                            ),
                        ));
                    }
                    if let Some(tag) = parse_added_tag(&event) {
                        if tag == stack_trace_tag {
                            stack_trace.push((
                                i32::MAX,
                                Self::new_stack_frame(
                                    i32::MAX,
                                    orig_ns,
                                    orig_fn,
                                    line,
                                    &mc_session.datapack,
                                ),
                            ));
                        }
                    }
                }
            }
        }

        trace!("Sending stack trace response");

        stack_trace.sort_by_key(|it| -it.0);

        let total_frames = Some(stack_trace.len() as i32);
        Ok(StackTraceResponseBody {
            stack_frames: stack_trace.into_iter().map(|it| it.1).collect(),
            total_frames,
        })
    }

    fn new_stack_frame(
        id: i32,
        orig_ns: &str,
        orig_fn: &str,
        line: i32,
        datapack: impl AsRef<Path>,
    ) -> StackFrame {
        StackFrame {
            id,
            name: format!("{}:{}:{}", orig_ns, orig_fn, line),
            source: Some(Source {
                name: None,
                path: Some(
                    datapack
                        .as_ref()
                        .join(&format!(
                            "data/{}/functions/{}.mcfunction",
                            orig_ns, orig_fn
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

fn get_command(request: &Request) -> String {
    let value = serde_json::to_value(request).unwrap();
    if let Value::Object(mut object) = value {
        let command = object.remove("command").unwrap();
        if let Value::String(command) = command {
            command
        } else {
            panic!("command must be a string");
        }
    } else {
        panic!("value must be an object");
    }
}

fn parse_stopped_tag(tag: &str) -> Option<(String, i32)> {
    let breakpoint_tag = tag.strip_prefix("stopped_at_breakpoint.")?;

    // -ns-+-orig_ns-+-orig_fn-+-line_number-
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

struct ClientSession {
    minecraft_session: Option<MinecraftSession>,
}

struct MinecraftSession {
    connection: MinecraftConnection,
    datapack: PathBuf,
    namespace: String,
}
impl MinecraftSession {
    fn inject_commands(&mut self, commands: Vec<String>) -> Result<(), PartialErrorResponse> {
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

/// Parse an event in the following format:
///
/// `[15:58:32] [Server thread/INFO]: [sample:main:2: Added 0 to [mcfd_depth] for 22466a74-94bd-458b-af97-3333c36d7b0b (now 1)]`
fn parse_scoreboard_value(event: &LogEvent, scoreboard: &str) -> Option<i32> {
    let suffix = event
        .message
        .strip_prefix(&format!("Added 0 to [{}] for ", scoreboard))?;
    const NOW: &str = " (now ";
    let index = suffix.find(NOW)?;
    let suffix = &suffix[index + NOW.len()..];
    let scoreboard_value = suffix.strip_suffix(')')?;
    scoreboard_value.parse().ok()
}

/// Parse an event in the following format:
///
/// `[16:09:59] [Server thread/INFO]: [sample:foo:2: Added tag 'mcfd_breakpoint' to sample:foo:2]`
fn parse_added_tag(event: &LogEvent) -> Option<&str> {
    let suffix = event.message.strip_prefix("Added tag '")?;
    const TO: &str = "' to ";
    let index = suffix.find(TO)?;
    Some(&suffix[..index])
}
