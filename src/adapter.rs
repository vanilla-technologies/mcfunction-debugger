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

pub mod utils;

use crate::{
    adapter::utils::{
        events_between, parse_function_path, BreakpointPosition, McfunctionStackFrame, StoppedData,
        StoppedEvent,
    },
    dap::{
        api::{DebugAdapter, DebugAdapterContext},
        error::{PartialErrorResponse, RequestError},
    },
    generator::{
        config::adapter::{AdapterConfig, BreakpointPositionInLine, LocalBreakpointPosition},
        generate_debug_datapack,
        parser::{
            command::{resource_location::ResourceLocation, CommandParser},
            parse_line, Line,
        },
        DebugDatapackMetadata,
    },
    installer::establish_connection,
};
use async_trait::async_trait;
use debug_adapter_protocol::{
    events::{
        Event, OutputCategory, OutputEventBody, StoppedEventBody, StoppedEventReason,
        TerminatedEventBody,
    },
    requests::{
        ContinueRequestArguments, EvaluateRequestArguments, InitializeRequestArguments,
        LaunchRequestArguments, NextRequestArguments, PathFormat, PauseRequestArguments,
        ScopesRequestArguments, SetBreakpointsRequestArguments, StackTraceRequestArguments,
        StepInRequestArguments, StepOutRequestArguments, TerminateRequestArguments,
        VariablesRequestArguments,
    },
    responses::{
        ContinueResponseBody, EvaluateResponseBody, ScopesResponseBody, SetBreakpointsResponseBody,
        StackTraceResponseBody, ThreadsResponseBody, VariablesResponseBody,
    },
    types::{Breakpoint, Capabilities, Scope, Thread, Variable},
    ProtocolMessage,
};
use futures::future::Either;
use log::trace;
use minect::{
    command::{
        logged_block_commands, named_logged_block_commands, query_scoreboard_command,
        summon_named_entity_command, AddTagOutput, QueryScoreboardOutput, SummonNamedEntityOutput,
    },
    log::LogEvent,
    Command, MinecraftConnection,
};
use multimap::MultiMap;
use std::{
    convert::TryFrom,
    io,
    path::{Path, PathBuf},
};
use tokio::{
    fs::{remove_dir_all, File},
    io::{AsyncBufReadExt, BufReader},
    sync::mpsc::UnboundedSender,
};
use tokio_stream::{wrappers::LinesStream, StreamExt};

const LISTENER_NAME: &'static str = "mcfunction_debugger";

struct ClientSession {
    lines_start_at_1: bool,
    columns_start_at_1: bool,
    path_format: PathFormat,
    mc_session: Option<MinecraftSession>,
    breakpoints: MultiMap<ResourceLocation, LocalBreakpointPosition>,
    parser: CommandParser,
}
impl ClientSession {
    fn get_line_offset(&self) -> usize {
        if self.lines_start_at_1 {
            0
        } else {
            1
        }
    }

    fn get_column_offset(&self) -> usize {
        if self.columns_start_at_1 {
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
    metadata: DebugDatapackMetadata,
    scopes: Vec<ScopeReference>,
    step_target_depth: Option<i32>,
    stopped_data: Option<StoppedData>,
}
impl MinecraftSession {
    fn inject_commands(&mut self, commands: Vec<Command>) -> Result<(), PartialErrorResponse> {
        inject_commands(&mut self.connection, commands)
            .map_err(|e| PartialErrorResponse::new(format!("Failed to inject commands: {}", e)))
    }

    fn replace_ns(&self, command: &str) -> String {
        command.replace("-ns-", &self.namespace)
    }

    pub fn setup_breakpoint_commands(
        &self,
        breakpoints: &MultiMap<ResourceLocation, LocalBreakpointPosition>,
    ) -> Vec<Command> {
        let mut commands = Vec::new();
        commands.push(Command::new(
            self.replace_ns("scoreboard players reset * -ns-_break"),
        ));
        for (function, breakpoints) in breakpoints.iter_all() {
            for breakpoint in breakpoints {
                commands.push(self.activate_breakpoint_command(function, &breakpoint));
            }
        }
        commands
    }

    pub(crate) fn activate_breakpoint_command(
        &self,
        fn_name: &ResourceLocation,
        position: &LocalBreakpointPosition,
    ) -> Command {
        Command::new(self.replace_ns(&format!(
            "scoreboard players set {} -ns-_break 1",
            self.metadata.get_breakpoint_score_holder(fn_name, position)
        )))
    }

    pub(crate) fn deactivate_breakpoint_command(
        &self,
        fn_name: &ResourceLocation,
        position: &LocalBreakpointPosition,
    ) -> Command {
        Command::new(self.replace_ns(&format!(
            "scoreboard players reset {} -ns-_break",
            self.metadata.get_breakpoint_score_holder(fn_name, position)
        )))
    }

    fn set_step_target_depth_command(&self, step_target_depth: Option<i32>) -> Command {
        if let Some(step_target_depth) = step_target_depth {
            Command::new(self.replace_ns(&format!(
                "scoreboard players set step_target -ns-_depth {}",
                step_target_depth
            )))
        } else {
            Command::new(self.replace_ns("scoreboard players reset step_target -ns-_depth"))
        }
    }

    async fn get_context_entity_id(&mut self, depth: i32) -> Result<i32, PartialErrorResponse> {
        let events = self.connection.add_listener();

        const START: &str = "get_context_entity_id.start";
        const END: &str = "get_context_entity_id.end";

        let scoreboard = self.replace_ns("-ns-_id");
        self.inject_commands(vec![
            Command::named(LISTENER_NAME, summon_named_entity_command(START)),
            Command::new(query_scoreboard_command(
                self.replace_ns(&format!(
                    "@e[\
                        type=area_effect_cloud,\
                        tag=-ns-_context,\
                        tag=-ns-_active,\
                        tag=-ns-_current,\
                        scores={{-ns-_depth={}}},\
                    ]",
                    depth
                )),
                &scoreboard,
            )),
            Command::named(LISTENER_NAME, summon_named_entity_command(END)),
        ])?;

        events_between(events, START, END)
            .filter_map(|event| event.output.parse::<QueryScoreboardOutput>().ok())
            .filter(|output| output.scoreboard == scoreboard)
            .map(|output| output.score)
            .next()
            .await
            .ok_or_else(|| PartialErrorResponse::new("Minecraft connection closed".to_string()))
    }

    fn get_cached_stack_trace(
        &self,
    ) -> Result<&Vec<McfunctionStackFrame>, RequestError<io::Error>> {
        Ok(&self
            .stopped_data
            .as_ref()
            .ok_or(PartialErrorResponse::new("Not stopped".to_string()))?
            .stack_trace)
    }

    async fn get_stack_trace(&mut self) -> io::Result<Vec<McfunctionStackFrame>> {
        const START: &str = "stack_trace.start";
        const END: &str = "stack_trace.end";
        let stack_trace_tag = self.replace_ns("-ns-_stack_trace");
        let depth_scoreboard = self.replace_ns("-ns-_depth");

        let events = self.connection.add_listener();

        let commands = vec![
            Command::named(LISTENER_NAME, summon_named_entity_command(START)),
            Command::new(self.replace_ns(&format!(
                "execute as @e[type=area_effect_cloud,tag=-ns-_function_call] run {}",
                query_scoreboard_command("@s", &depth_scoreboard)
            ))),
            Command::new(self.replace_ns(&format!(
                "execute as @e[type=area_effect_cloud,tag=-ns-_breakpoint] run tag @s add {}",
                stack_trace_tag
            ))),
            Command::new(self.replace_ns(&format!(
                "execute as @e[type=area_effect_cloud,tag=-ns-_breakpoint] run tag @s remove {}",
                stack_trace_tag
            ))),
            Command::named(LISTENER_NAME, summon_named_entity_command(END)),
        ];
        inject_commands(&mut self.connection, commands)?;

        let mut stack_trace = Vec::new();
        let mut events = events_between(events, START, END);
        while let Some(event) = events.next().await {
            if let Ok(location) = event.executor.parse() {
                let id = if let Some(output) = event
                    .output
                    .parse::<QueryScoreboardOutput>()
                    .ok()
                    .filter(|output| output.scoreboard == depth_scoreboard)
                {
                    output.score // depth
                } else if let Some(_) = event
                    .output
                    .parse::<AddTagOutput>()
                    .ok()
                    .filter(|output| output.tag == stack_trace_tag)
                {
                    stack_trace.len() as i32 // Breakpoint
                } else {
                    continue; // Shouldn't actually happen
                };
                stack_trace.push(McfunctionStackFrame { id, location });
            }
        }
        stack_trace.sort_by_key(|it| -it.id);
        Ok(stack_trace)
    }

    async fn uninstall_datapack(&mut self) -> io::Result<()> {
        let events = self.connection.add_listener();

        let uninstalled = format!("{}.uninstalled", LISTENER_NAME);
        inject_commands(
            &mut self.connection,
            vec![
                Command::new("function debug:uninstall"),
                Command::new(summon_named_entity_command(&uninstalled)),
            ],
        )?;

        trace!("Waiting for datapack to be uninstalled...");
        events
            .filter_map(|e| e.output.parse::<SummonNamedEntityOutput>().ok())
            .filter(|o| o.name == uninstalled)
            .next()
            .await;
        trace!("Datapack is uninstalled");

        remove_dir_all(&self.output_path).await?;
        Ok(())
    }
}

pub(crate) fn inject_commands(
    connection: &mut MinecraftConnection,
    commands: Vec<Command>,
) -> io::Result<()> {
    trace!(
        "Injecting commands:{}",
        commands
            .iter()
            .map(|it| it.get_command())
            .fold(String::new(), |joined, command| joined + "\n" + command)
    );
    connection.execute_commands(commands)?;
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

pub struct McfunctionDebugAdapter {
    message_sender: UnboundedSender<Either<ProtocolMessage, LogEvent>>,
    client_session: Option<ClientSession>,
}
impl McfunctionDebugAdapter {
    pub fn new(message_sender: UnboundedSender<Either<ProtocolMessage, LogEvent>>) -> Self {
        McfunctionDebugAdapter {
            message_sender,
            client_session: None,
        }
    }

    async fn on_stopped(
        &mut self,
        event: StoppedEvent,
        context: &mut (impl DebugAdapterContext + Send),
    ) -> io::Result<()> {
        if let Some(client_session) = &mut self.client_session {
            if let Some(mc_session) = &mut client_session.mc_session {
                let stack_trace = mc_session.get_stack_trace().await?;
                let current_depth = stack_trace.len() as i32 - 1;

                mc_session.stopped_data = Some(StoppedData {
                    position: event.position,
                    stack_trace,
                });

                let reason = match mc_session.step_target_depth {
                    Some(step_target_depth) if current_depth <= step_target_depth => {
                        StoppedEventReason::Step
                    }
                    _ => StoppedEventReason::Breakpoint,
                };

                let event = StoppedEventBody::builder()
                    .reason(reason)
                    .thread_id(Some(MAIN_THREAD_ID))
                    .build();
                context.fire_event(event);
            }
        }

        Ok(())
    }

    async fn on_exited(
        &mut self,
        context: &mut (impl DebugAdapterContext + Send),
    ) -> io::Result<()> {
        if let Some(client_session) = &mut self.client_session {
            if let Some(mc_session) = &mut client_session.mc_session {
                mc_session.uninstall_datapack().await?;

                context.fire_event(TerminatedEventBody::builder().build());
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
        mc_session: &mut Option<MinecraftSession>,
    ) -> Result<&mut MinecraftSession, PartialErrorResponse> {
        mc_session.as_mut().ok_or_else(|| PartialErrorResponse {
            message: "Not launched or attached".to_string(),
            details: None,
        })
    }

    async fn continue_internal(
        &mut self,
        depth_offset: Option<i32>,
    ) -> Result<(), RequestError<io::Error>> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let mc_session = Self::unwrap_minecraft_session(&mut client_session.mc_session)?;

        let stopped_data = mc_session
            .stopped_data
            .as_ref()
            .ok_or(PartialErrorResponse::new("Not stopped".to_string()))?;

        let current_depth = stopped_data.stack_trace.len() as i32 - 1;
        let step_target_depth = depth_offset.map(|depth_offset| current_depth + depth_offset);
        mc_session.step_target_depth = step_target_depth;

        // Continue must be scheduled to ensure it runs before suspended schedules
        let commands = Vec::from_iter([
            mc_session.set_step_target_depth_command(step_target_depth),
            Command::new(mc_session.replace_ns("schedule function -ns-:prepare_resume 1t")),
            Command::new(mc_session.replace_ns(&format!(
                "schedule function -ns-:{}/{}/continue_current_iteration_at_{}_{} 1t",
                stopped_data.position.function.namespace(),
                stopped_data.position.function.path(),
                stopped_data.position.line_number,
                stopped_data.position.position_in_line,
            ))),
        ]);

        mc_session.inject_commands(commands)?;
        mc_session.stopped_data = None;
        mc_session.scopes.clear();

        Ok(())
    }
}

#[async_trait]
impl DebugAdapter for McfunctionDebugAdapter {
    type Message = LogEvent;
    type CustomError = io::Error;

    async fn handle_other_message(
        &mut self,
        msg: Self::Message,
        mut context: impl DebugAdapterContext + Send,
    ) -> Result<(), Self::CustomError> {
        trace!(
            "Received message from Minecraft by {}: {}",
            msg.executor,
            msg.output
        );
        if let Ok(output) = msg.output.parse::<SummonNamedEntityOutput>() {
            if let Ok(event) = output.name.parse() {
                self.on_stopped(event, &mut context).await?;
            }
            if output.name == "exited" {
                self.on_exited(&mut context).await?;
            }
        }
        Ok(())
    }

    async fn continue_(
        &mut self,
        _args: ContinueRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<ContinueResponseBody, RequestError<Self::CustomError>> {
        self.continue_internal(None).await?;

        Ok(ContinueResponseBody::builder().build())
    }

    async fn evaluate(
        &mut self,
        _args: EvaluateRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<EvaluateResponseBody, RequestError<Self::CustomError>> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let _mc_session = Self::unwrap_minecraft_session(&mut client_session.mc_session)?;

        Err(RequestError::Respond(PartialErrorResponse::new(
            "Not supported yet, see: \
            https://github.com/vanilla-technologies/mcfunction-debugger/issues/68"
                .to_string(),
        )))
    }

    async fn initialize(
        &mut self,
        args: InitializeRequestArguments,
        mut context: impl DebugAdapterContext + Send,
    ) -> Result<Capabilities, RequestError<Self::CustomError>> {
        let parser = CommandParser::default()
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))
            .map_err(Self::map_custom_error)?;
        self.client_session = Some(ClientSession {
            lines_start_at_1: args.lines_start_at_1,
            columns_start_at_1: args.columns_start_at_1,
            path_format: args.path_format,
            mc_session: None,
            breakpoints: MultiMap::new(),
            parser,
        });

        context.fire_event(Event::Initialized);

        Ok(Capabilities::builder()
            .supports_cancel_request(true)
            .supports_terminate_request(true)
            .build())
    }

    async fn launch(
        &mut self,
        args: LaunchRequestArguments,
        context: impl DebugAdapterContext + Send,
    ) -> Result<(), RequestError<Self::CustomError>> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;

        let config = get_config(&args)?;

        let mut connection = establish_connection(
            &config.minecraft_world_dir,
            &config.minecraft_log_file,
            context,
        )
        .await?;

        let mut events = connection.add_named_listener(LISTENER_NAME);
        let message_sender = self.message_sender.clone();
        tokio::spawn(async move {
            while let Some(event) = events.next().await {
                if let Err(_) = message_sender.send(Either::Right(event)) {
                    break;
                }
            }
        });

        let namespace = "mcfd".to_string(); // Hardcoded in installer as well
        let debug_datapack_name = format!("debug-{}", config.datapack_name);
        let output_path = config
            .minecraft_world_dir
            .join("datapacks")
            .join(&debug_datapack_name);

        let datapack = config.datapack.to_path_buf();

        let generator_config = crate::generator::config::Config {
            namespace: &namespace,
            shadow: false,
            adapter: Some(AdapterConfig {
                adapter_listener_name: LISTENER_NAME,
            }),
        };
        let _ = remove_dir_all(&output_path).await;
        let metadata = generate_debug_datapack(&datapack, &output_path, &generator_config)
            .await
            .map_err(|e| {
                PartialErrorResponse::new(format!("Failed to generate debug datapack: {}", e))
            })?;

        let mut mc_session = MinecraftSession {
            connection,
            datapack,
            namespace,
            output_path,
            metadata,
            scopes: Vec::new(),
            step_target_depth: None,
            stopped_data: None,
        };

        let commands = vec![
            Command::new("reload"),
            Command::new(format!("datapack enable \"file/{}\"", debug_datapack_name)),
        ];
        mc_session.inject_commands(commands)?;
        // After loading the datapack we must wait one tick for it to install itself
        let mut commands = mc_session.setup_breakpoint_commands(&client_session.breakpoints);
        // By scheduling this function call we have a defined execution position
        commands.push(Command::new(format!(
            "schedule function debug:{}/{} 1t",
            config.function.namespace(),
            config.function.path(),
        )));
        mc_session.inject_commands(commands)?;

        client_session.mc_session = Some(mc_session);
        Ok(())
    }

    async fn next(
        &mut self,
        _args: NextRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<(), RequestError<Self::CustomError>> {
        self.continue_internal(Some(0)).await
    }

    async fn pause(
        &mut self,
        _args: PauseRequestArguments,
        mut context: impl DebugAdapterContext + Send,
    ) -> Result<(), RequestError<Self::CustomError>> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let _mc_session = Self::unwrap_minecraft_session(&mut client_session.mc_session)?;

        let event = OutputEventBody::builder()
            .category(OutputCategory::Important)
            .output("Minecraft cannot be paused".to_string())
            .build();
        context.fire_event(event);

        Err(RequestError::Respond(PartialErrorResponse::new(
            "Minecraft cannot be paused".to_string(),
        )))
    }

    async fn scopes(
        &mut self,
        args: ScopesRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<ScopesResponseBody, RequestError<Self::CustomError>> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let mc_session = Self::unwrap_minecraft_session(&mut client_session.mc_session)?;

        let mut scopes = Vec::new();
        let is_server_context = mc_session.get_context_entity_id(args.frame_id).await? == 0;
        if !is_server_context {
            scopes.push(create_selected_entity_scores_scope(mc_session, args));
        }
        Ok(ScopesResponseBody::builder().scopes(scopes).build().into())
    }

    async fn set_breakpoints(
        &mut self,
        args: SetBreakpointsRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<SetBreakpointsResponseBody, RequestError<Self::CustomError>> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;

        let offset = client_session.get_line_offset();
        let path = match client_session.path_format {
            PathFormat::Path => args.source.path.as_ref().ok_or_else(|| {
                PartialErrorResponse::new("Missing argument source.path".to_string())
            })?,
            PathFormat::URI => todo!("Implement path URIs"),
        };
        let (_datapack, function) = parse_function_path(path.as_ref())
            .map_err(|e| PartialErrorResponse::new(format!("Argument source.path {}", e)))?;

        let breakpoints = args
            .breakpoints
            .iter()
            .map(|source_breakpoint| (function.clone(), source_breakpoint.line as usize + offset))
            .collect::<Vec<_>>();

        let mut response = Vec::new();
        let old_breakpoints = client_session
            .breakpoints
            .remove(&function)
            .unwrap_or_default();
        let mut new_breakpoints = Vec::with_capacity(breakpoints.len());
        for (function, line_number) in breakpoints.into_iter() {
            let verified = verify_breakpoint(&client_session.parser, path, line_number)
                .await
                .map_err(|e| {
                    PartialErrorResponse::new(format!(
                        "Failed to verify breakpoint {}:{}: {}",
                        function, line_number, e
                    ))
                })?;
            new_breakpoints.push(LocalBreakpointPosition {
                line_number,
                position_in_line: BreakpointPositionInLine::Breakpoint,
            });
            response.push(
                Breakpoint::builder()
                    .id(None)
                    .verified(verified)
                    .line(Some((line_number - offset) as i32))
                    .build(),
            );
        }

        client_session
            .breakpoints
            .insert_many(function.clone(), new_breakpoints);
        // Unwrap is safe, because we just inserted the value
        let new_breakpoints = client_session.breakpoints.get_vec(&function).unwrap();

        if let Some(mc_session) = client_session.mc_session.as_mut() {
            let mut commands = Vec::new();
            for breakpoint in &old_breakpoints {
                commands.push(mc_session.deactivate_breakpoint_command(&function, &breakpoint));
            }
            for breakpoint in new_breakpoints {
                commands.push(mc_session.activate_breakpoint_command(&function, &breakpoint));
            }
            if args.source_modified && old_breakpoints.len() == new_breakpoints.len() {
                commands.extend(get_move_breakpoint_commands(
                    old_breakpoints.iter().map(|breakpoint| {
                        BreakpointPosition::from_breakpoint(function.clone(), &breakpoint)
                    }),
                    new_breakpoints.iter().map(|breakpoint| {
                        BreakpointPosition::from_breakpoint(function.clone(), &breakpoint)
                    }),
                    &mc_session.namespace,
                ));
            }
            mc_session.inject_commands(commands)?;
        }

        Ok(SetBreakpointsResponseBody::builder()
            .breakpoints(response)
            .build())
    }

    async fn stack_trace(
        &mut self,
        _args: StackTraceRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<StackTraceResponseBody, RequestError<Self::CustomError>> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let get_line_offset = client_session.get_line_offset();
        let get_column_offset = client_session.get_column_offset();
        let mc_session = Self::unwrap_minecraft_session(&mut client_session.mc_session)?;

        let stack_trace = mc_session
            .get_cached_stack_trace()?
            .into_iter()
            .map(|it| it.to_stack_frame(&mc_session.datapack, get_line_offset, get_column_offset))
            .collect::<Vec<_>>();

        Ok(StackTraceResponseBody::builder()
            .total_frames(Some(stack_trace.len() as i32))
            .stack_frames(stack_trace)
            .build())
    }

    async fn terminate(
        &mut self,
        _args: TerminateRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<(), RequestError<Self::CustomError>> {
        if let Some(client_session) = &mut self.client_session {
            if let Some(mc_session) = &mut client_session.mc_session {
                mc_session.inject_commands(vec![Command::new("function debug:stop")])?;
            }
        }
        Ok(())
    }

    async fn step_in(
        &mut self,
        _args: StepInRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<(), RequestError<Self::CustomError>> {
        self.continue_internal(Some(1)).await
    }

    async fn step_out(
        &mut self,
        _args: StepOutRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<(), RequestError<Self::CustomError>> {
        self.continue_internal(Some(-1)).await
    }

    async fn threads(
        &mut self,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<ThreadsResponseBody, RequestError<Self::CustomError>> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let _mc_session = Self::unwrap_minecraft_session(&mut client_session.mc_session)?;

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
        _context: impl DebugAdapterContext + Send,
    ) -> Result<VariablesResponseBody, RequestError<Self::CustomError>> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let mc_session = Self::unwrap_minecraft_session(&mut client_session.mc_session)?;

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

        const START: &str = "variables.start";
        const END: &str = "variables.end";

        match scope.kind {
            ScopeKind::SelectedEntityScores => {
                let events = mc_session.connection.add_listener();

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
                let mut commands = Vec::new();
                commands.extend(named_logged_block_commands(
                    LISTENER_NAME,
                    &summon_named_entity_command(START),
                ));
                commands.extend(logged_block_commands(&decrement_ids));
                commands.push(mc_session.replace_ns("function -ns-:log_scores"));
                commands.extend(logged_block_commands(&increment_ids));
                commands.extend(named_logged_block_commands(
                    LISTENER_NAME,
                    &summon_named_entity_command(END),
                ));
                let commands = commands.into_iter().map(Command::new).collect();
                mc_session.inject_commands(commands)?;

                let variables = events_between(events, START, END)
                    .filter_map(|event| event.output.parse::<QueryScoreboardOutput>().ok())
                    .map(|output| {
                        Variable::builder()
                            .name(output.scoreboard)
                            .value(output.score.to_string())
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

struct Config<'l> {
    datapack: &'l Path,
    datapack_name: &'l str,
    function: ResourceLocation,
    minecraft_world_dir: &'l Path,
    minecraft_log_file: &'l Path,
}

fn get_config(args: &LaunchRequestArguments) -> Result<Config, PartialErrorResponse> {
    let program = get_path(&args, "program")?;

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

    let minecraft_world_dir = get_path(&args, "minecraftWorldDir")?;
    let minecraft_log_file = get_path(&args, "minecraftLogFile")?;
    Ok(Config {
        datapack,
        datapack_name,
        function,
        minecraft_world_dir,
        minecraft_log_file,
    })
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
        .build()
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
        return Ok(is_command(line));
    } else {
        Ok(false)
    }
}
fn get_move_breakpoint_commands(
    old_positions: impl ExactSizeIterator<Item = BreakpointPosition>,
    new_positions: impl ExactSizeIterator<Item = BreakpointPosition>,
    namespace: &str,
) -> Vec<Command> {
    let tmp_tag = format!("{}_tmp", namespace);
    let breakpoint_tag = format!("{}_breakpoint", namespace);
    let mut commands = Vec::new();
    for (old_position, new_position) in old_positions.zip(new_positions) {
        if old_position != new_position {
            let old_tag = format!("{}+{}", namespace, old_position);
            let new_tag = format!("{}+{}", namespace, new_position);
            commands.push(Command::new(format!(
                "tag @e[tag={},tag={},tag=!{}] add {}",
                breakpoint_tag, old_tag, tmp_tag, new_tag,
            )));
            commands.push(Command::new(format!(
                "tag @e[tag={},tag={}] add {}",
                breakpoint_tag, old_tag, tmp_tag
            )));
            commands.push(Command::new(format!(
                "tag @e[tag={},tag={},tag={}] remove {}",
                breakpoint_tag, old_tag, new_tag, old_tag
            )));
        }
    }
    commands.push(Command::new(format!(
        "tag @e[tag={},tag={}] remove {}",
        breakpoint_tag, tmp_tag, tmp_tag
    )));
    commands
}

fn is_command(line: Line) -> bool {
    !matches!(line, Line::Empty | Line::Comment | Line::Breakpoint)
}
