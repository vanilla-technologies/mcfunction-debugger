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

pub mod utils;

use crate::{
    adapter::utils::{
        events_between, generate_datapack, is_temporary, parse_function_path,
        to_stopped_event_reason, StackFrameLocation, StoppedEvent,
    },
    error::{PartialErrorResponse, RequestError},
    installer::establish_connection,
    DebugAdapter, DebugAdapterContext,
};
use async_trait::async_trait;
use debug_adapter_protocol::{
    events::{Event, OutputCategory, OutputEventBody, StoppedEventBody, TerminatedEventBody},
    requests::{
        ContinueRequestArguments, EvaluateRequestArguments, InitializeRequestArguments,
        LaunchRequestArguments, PathFormat, PauseRequestArguments, ScopesRequestArguments,
        SetBreakpointsRequestArguments, StackTraceRequestArguments, StepOutRequestArguments,
        TerminateRequestArguments, VariablesRequestArguments,
    },
    responses::{
        ContinueResponseBody, EvaluateResponseBody, ScopesResponseBody, SetBreakpointsResponseBody,
        StackTraceResponseBody, ThreadsResponseBody, VariablesResponseBody,
    },
    types::{Breakpoint, Capabilities, Scope, Source, StackFrame, Thread, Variable},
    ProtocolMessage,
};
use futures::future::Either;
use log::trace;
use mcfunction_debugger::{
    parser::{
        command::{resource_location::ResourceLocation, CommandParser},
        parse_line, Line,
    },
    BreakpointKind, LocalBreakpoint,
};
use minect::{
    log::{
        enable_logging_command, logged_command, named_logged_command, query_scoreboard_command,
        reset_logging_command, summon_named_entity_command, AddTagOutput, LogEvent,
        QueryScoreboardOutput, SummonNamedEntityOutput,
    },
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

use self::utils::{can_resume_from, Position, SuspensionPositionInLine};

const LISTENER_NAME: &'static str = "mcfunction_debugger";

struct ClientSession {
    lines_start_at_1: bool,
    columns_start_at_1: bool,
    path_format: PathFormat,
    minecraft_session: Option<MinecraftSession>,
    breakpoints: MultiMap<ResourceLocation, LocalBreakpoint>,
    generated_breakpoints: MultiMap<ResourceLocation, LocalBreakpoint>,
    stopped_event: Option<StoppedEvent>,
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
    scopes: Vec<ScopeReference>,
}
impl MinecraftSession {
    fn inject_commands(&mut self, commands: Vec<Command>) -> Result<(), PartialErrorResponse> {
        inject_commands(&mut self.connection, commands)
            .map_err(|e| PartialErrorResponse::new(format!("Failed to inject commands: {}", e)))
    }

    fn replace_ns(&self, command: &str) -> String {
        command.replace("-ns-", &self.namespace)
    }

    async fn get_context_entity_id(&mut self, depth: i32) -> Result<i32, PartialErrorResponse> {
        let events = self.connection.add_listener();

        const START: &str = "get_context_entity_id.start";
        const END: &str = "get_context_entity_id.end";

        let scoreboard = self.replace_ns("-ns-_id");
        self.inject_commands(vec![
            Command::new(enable_logging_command()),
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
            Command::new(reset_logging_command()),
        ])?;

        events_between(events, START, END)
            .filter_map(|event| event.output.parse::<QueryScoreboardOutput>().ok())
            .filter(|output| output.scoreboard == scoreboard)
            .map(|output| output.score)
            .next()
            .await
            .ok_or_else(|| PartialErrorResponse::new("Minecraft connection closed".to_string()))
    }

    async fn get_stack_trace(&mut self) -> Result<Vec<McfunctionStackFrame>, PartialErrorResponse> {
        const START: &str = "stack_trace.start";
        const END: &str = "stack_trace.end";
        let stack_trace_tag = self.replace_ns("-ns-_stack_trace");
        let depth_scoreboard = self.replace_ns("-ns-_depth");

        let events = self.connection.add_listener();

        self.inject_commands(vec![
            Command::new(enable_logging_command()),
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
            Command::new(reset_logging_command()),
        ])?;

        let mut stack_trace = Vec::new();
        let mut events = events_between(events, START, END);
        while let Some(event) = events.next().await {
            if let Some(location) = StackFrameLocation::parse(&event.executor) {
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
                Command::new(enable_logging_command()),
                Command::new(summon_named_entity_command(&uninstalled)),
                Command::new(reset_logging_command()),
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
    connection.inject_commands(commands)?;
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
    ) {
        let reason = event.reason;
        if let Some(client_session) = &mut self.client_session {
            client_session.stopped_event = Some(event);
        }

        let event = StoppedEventBody::builder()
            .reason(to_stopped_event_reason(reason))
            .thread_id(Some(MAIN_THREAD_ID))
            .build();
        context.fire_event(event);
    }

    async fn on_exited(
        &mut self,
        context: &mut (impl DebugAdapterContext + Send),
    ) -> io::Result<()> {
        if let Some(client_session) = &mut self.client_session {
            if let Some(minecraft_session) = &mut client_session.minecraft_session {
                minecraft_session.uninstall_datapack().await?;
            }
        }

        context.fire_event(TerminatedEventBody::builder().build());

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

    async fn continue_internal(
        &mut self,
        temporary_breakpoints: Vec<(ResourceLocation, LocalBreakpoint)>,
    ) -> Result<(), RequestError<io::Error>> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let mc_session = Self::unwrap_minecraft_session(&mut client_session.minecraft_session)?;

        if let Some(stopped_event) = client_session.stopped_event.as_ref() {
            let mut dirty = false;

            // Remove all temporary generated breakpoints
            for (_key, values) in client_session.generated_breakpoints.iter_all_mut() {
                values.retain(|it| {
                    let should_remove = is_temporary(it.kind);
                    if should_remove {
                        dirty = true;
                    }
                    !should_remove
                });
            }

            for (function, breakpoint) in temporary_breakpoints {
                client_session
                    .generated_breakpoints
                    .insert(function, breakpoint);
                dirty = true;
            }

            client_session.generated_breakpoints.insert(
                stopped_event.position.function.clone(),
                LocalBreakpoint {
                    line_number: stopped_event.position.line_number,
                    kind: BreakpointKind::Continue {
                        after_function: match stopped_event.position.position_in_line {
                            SuspensionPositionInLine::Breakpoint => false,
                            SuspensionPositionInLine::AfterFunction => true,
                        },
                    },
                },
            );
            // If there isn't already a breakpoint that can resume we need to load the continue point
            if !can_resume_from(&client_session.breakpoints, &stopped_event.position) {
                dirty = true;
            }

            let mut commands = Vec::new();

            if dirty {
                generate_datapack(
                    mc_session,
                    &client_session.breakpoints,
                    &client_session.generated_breakpoints,
                )
                .await?;
                commands.push(Command::new("reload"));
            };

            commands.push(Command::new("function debug:resume"));
            mc_session.inject_commands(commands)?;
            client_session.stopped_event = None;
            mc_session.scopes.clear();
        }

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
        if let Ok(output) = msg.output.parse::<AddTagOutput>() {
            if output.entity == LISTENER_NAME {
                if let Ok(event) = output.tag.parse() {
                    self.on_stopped(event, &mut context).await;
                }
                if output.tag == "exited" {
                    self.on_exited(&mut context).await?;
                }
            }
        }
        Ok(())
    }

    async fn continue_(
        &mut self,
        _args: ContinueRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<ContinueResponseBody, RequestError<Self::CustomError>> {
        self.continue_internal(Vec::new()).await?;

        Ok(ContinueResponseBody::builder().build())
    }

    async fn evaluate(
        &mut self,
        _args: EvaluateRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<EvaluateResponseBody, RequestError<Self::CustomError>> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let _mc_session = Self::unwrap_minecraft_session(&mut client_session.minecraft_session)?;

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
            minecraft_session: None,
            breakpoints: MultiMap::new(),
            generated_breakpoints: MultiMap::new(),
            stopped_event: None,
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

        let mut minecraft_session = MinecraftSession {
            connection,
            datapack: config.datapack.to_path_buf(),
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

        minecraft_session.inject_commands(vec![
            Command::new("reload"),
            Command::new(format!("datapack enable \"file/{}\"", debug_datapack_name)),
            // After loading the datapack we must wait one tick for it to install itself
            Command::new(format!(
                "schedule function debug:{}/{} 1t",
                config.function.namespace(),
                config.function.path(),
            )),
        ])?;

        client_session.minecraft_session = Some(minecraft_session);
        Ok(())
    }

    async fn pause(
        &mut self,
        _args: PauseRequestArguments,
        mut context: impl DebugAdapterContext + Send,
    ) -> Result<(), RequestError<Self::CustomError>> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let _mc_session = Self::unwrap_minecraft_session(&mut client_session.minecraft_session)?;

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
        let mc_session = Self::unwrap_minecraft_session(&mut client_session.minecraft_session)?;

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
        for (i, (function, line_number)) in breakpoints.into_iter().enumerate() {
            let id = (i + client_session.breakpoints.len()) as i32;
            let verified = verify_breakpoint(&client_session.parser, path, line_number)
                .await
                .map_err(|e| {
                    PartialErrorResponse::new(format!(
                        "Failed to verify breakpoint {}:{}: {}",
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
                    .line(Some((line_number - offset) as i32))
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
            let mut commands = vec![Command::new("reload")];
            if args.source_modified && old_breakpoints.len() == new_breakpoints.len() {
                commands.extend(get_move_breakpoint_commands(
                    old_breakpoints.iter().map(|it| Position {
                        function: function.clone(),
                        line_number: it.line_number,
                        position_in_line: it.kind.into(),
                    }),
                    new_breakpoints.iter().map(|it| Position {
                        function: function.clone(),
                        line_number: it.line_number,
                        position_in_line: it.kind.into(),
                    }),
                    &minecraft_session.namespace,
                ));
            }
            minecraft_session.inject_commands(commands)?;
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
        let mc_session = Self::unwrap_minecraft_session(&mut client_session.minecraft_session)?;

        let stack_trace = mc_session
            .get_stack_trace()
            .await?
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
            if let Some(minecraft_session) = &mut client_session.minecraft_session {
                minecraft_session.inject_commands(vec![Command::new("function debug:stop")])?;
            }
        }
        Ok(())
    }

    async fn step_out(
        &mut self,
        _args: StepOutRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<(), RequestError<Self::CustomError>> {
        let client_session = Self::unwrap_client_session(&mut self.client_session)?;
        let mc_session = Self::unwrap_minecraft_session(&mut client_session.minecraft_session)?;

        let mut temporary_breakpoints = Vec::new();

        let stack_trace = mc_session.get_stack_trace().await?;
        if stack_trace.len() > 1 {
            let caller = &stack_trace[1];
            let path = mc_session
                .datapack
                .join("data")
                .join(caller.location.function.mcfunction_path());

            let line_number = find_step_target_line_number(
                path,
                caller.location.line_number,
                &client_session.parser,
            )
            .await?;

            temporary_breakpoints.push((
                caller.location.function.clone(),
                LocalBreakpoint {
                    line_number,
                    kind: BreakpointKind::Step {
                        after_function: line_number == caller.location.line_number,
                    },
                },
            ));
        }

        self.continue_internal(temporary_breakpoints).await?;

        Ok(())
    }

    async fn threads(
        &mut self,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<ThreadsResponseBody, RequestError<Self::CustomError>> {
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
        _context: impl DebugAdapterContext + Send,
    ) -> Result<VariablesResponseBody, RequestError<Self::CustomError>> {
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
                mc_session.inject_commands(vec![
                    Command::new(logged_command(enable_logging_command())),
                    Command::new(named_logged_command(
                        LISTENER_NAME,
                        summon_named_entity_command(START),
                    )),
                    Command::new(logged_command(decrement_ids)),
                    Command::new(mc_session.replace_ns("function -ns-:log_scores")),
                    Command::new(logged_command(increment_ids)),
                    Command::new(named_logged_command(
                        LISTENER_NAME,
                        summon_named_entity_command(END),
                    )),
                    Command::new(logged_command(reset_logging_command())),
                ])?;

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

async fn find_step_target_line_number(
    path: impl AsRef<Path>,
    after_line_index: usize,
    parser: &CommandParser,
) -> Result<usize, RequestError<io::Error>> {
    let file = File::open(&path).await.map_err(|e| {
        PartialErrorResponse::new(format!(
            "Failed to open file {}: {}",
            path.as_ref().display(),
            e
        ))
    })?;
    let lines = BufReader::new(file).lines();
    let mut lines = LinesStream::new(lines).skip(after_line_index);
    let mut line_index = after_line_index;
    while let Some(line) = lines.next().await {
        line_index += 1;
        let line = line.map_err(|e| {
            PartialErrorResponse::new(format!(
                "Failed to read file {}: {}",
                path.as_ref().display(),
                e
            ))
        })?;
        let line = parse_line(parser, &line, false);
        if is_command(line) {
            return Ok(line_index);
        }
    }
    if line_index == after_line_index {
        Ok(line_index) // This is then the last line of the file
    } else {
        Ok(after_line_index + 1)
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
    old_positions: impl ExactSizeIterator<Item = Position>,
    new_positions: impl ExactSizeIterator<Item = Position>,
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

struct McfunctionStackFrame {
    id: i32,
    location: StackFrameLocation,
}
impl McfunctionStackFrame {
    fn to_stack_frame(
        &self,
        datapack: impl AsRef<Path>,
        line_offset: usize,
        column_offset: usize,
    ) -> StackFrame {
        let path = datapack
            .as_ref()
            .join("data")
            .join(self.location.function.mcfunction_path())
            .display()
            .to_string();
        StackFrame::builder()
            .id(self.id)
            .name(self.location.get_name())
            .source(Some(Source::builder().path(Some(path)).build()))
            .line((self.location.line_number - line_offset) as i32)
            .column((self.location.column_number - column_offset) as i32)
            .build()
    }
}
