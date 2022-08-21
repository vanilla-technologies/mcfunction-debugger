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

pub mod adapter;
pub mod codec;
mod error;
mod minecraft;

use async_trait::async_trait;
use debug_adapter_protocol::{
    requests::{
        ContinueRequestArguments, DisconnectRequestArguments, EvaluateRequestArguments,
        InitializeRequestArguments, LaunchRequestArguments, PauseRequestArguments, Request,
        ScopesRequestArguments, SetBreakpointsRequestArguments, StackTraceRequestArguments,
        TerminateRequestArguments, VariablesRequestArguments,
    },
    responses::{
        ContinueResponseBody, ErrorResponse, EvaluateResponseBody, Response, ScopesResponseBody,
        SetBreakpointsResponseBody, StackTraceResponseBody, SuccessResponse, ThreadsResponseBody,
        VariablesResponseBody,
    },
    types::Capabilities,
    ProtocolMessage, ProtocolMessageContent, SequenceNumber,
};
use error::{DapError, PartialErrorResponse};
use futures::{Sink, SinkExt};
use log::trace;
use serde_json::Value;

pub struct MessageWriter<O>
where
    O: Sink<ProtocolMessage> + Unpin,
{
    seq: SequenceNumber,
    output: O,
}

impl<O> MessageWriter<O>
where
    O: Sink<ProtocolMessage> + Unpin,
{
    pub fn new(output: O) -> MessageWriter<O> {
        MessageWriter { seq: 0, output }
    }

    pub async fn respond(
        &mut self,
        request_seq: SequenceNumber,
        result: Result<SuccessResponse, ErrorResponse>,
    ) -> Result<(), O::Error> {
        self.write_msg(ProtocolMessageContent::Response(Response {
            request_seq,
            result,
        }))
        .await
    }

    pub async fn write_msg(
        &mut self,
        content: impl Into<ProtocolMessageContent>,
    ) -> Result<(), O::Error> {
        self.seq += 1;
        let msg = ProtocolMessage::new(self.seq, content);
        trace!("Sending message to client: {}", msg);
        self.output.send(msg).await
    }
}

pub fn get_command(request: &Request) -> String {
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

#[async_trait]
trait DebugAdapter {
    async fn handle_client_request(
        &mut self,
        request: Request,
    ) -> Result<SuccessResponse, DapError> {
        match request {
            Request::ConfigurationDone => self
                .configuration_done()
                .await
                .map(|()| SuccessResponse::ConfigurationDone),
            Request::Continue(args) => self.continue_(args).await.map(SuccessResponse::Continue),
            Request::Disconnect(args) => self
                .disconnect(args)
                .await
                .map(|()| SuccessResponse::Disconnect),
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

    async fn configuration_done(&mut self) -> Result<(), DapError> {
        Err(DapError::Respond(PartialErrorResponse::new(
            "Unsupported request 'configurationDone'".to_string(),
        )))
    }

    async fn continue_(
        &mut self,
        _args: ContinueRequestArguments,
    ) -> Result<ContinueResponseBody, DapError>;

    async fn disconnect(&mut self, _args: DisconnectRequestArguments) -> Result<(), DapError> {
        Ok(())
    }

    async fn evaluate(
        &mut self,
        _args: EvaluateRequestArguments,
    ) -> Result<EvaluateResponseBody, DapError> {
        Err(DapError::Respond(PartialErrorResponse::new(
            "Unsupported request 'evaluate'".to_string(),
        )))
    }

    async fn initialize(
        &mut self,
        _args: InitializeRequestArguments,
    ) -> Result<Capabilities, DapError> {
        Err(DapError::Respond(PartialErrorResponse::new(
            "Unsupported request 'initialize'".to_string(),
        )))
    }

    async fn launch(&mut self, _args: LaunchRequestArguments) -> Result<(), DapError> {
        Err(DapError::Respond(PartialErrorResponse::new(
            "Unsupported request 'launch'".to_string(),
        )))
    }

    async fn pause(&mut self, _args: PauseRequestArguments) -> Result<(), DapError> {
        Err(DapError::Respond(PartialErrorResponse::new(
            "Unsupported request 'pause'".to_string(),
        )))
    }

    async fn scopes(
        &mut self,
        _args: ScopesRequestArguments,
    ) -> Result<ScopesResponseBody, DapError> {
        Err(DapError::Respond(PartialErrorResponse::new(
            "Unsupported request 'scopes'".to_string(),
        )))
    }

    async fn set_breakpoints(
        &mut self,
        _args: SetBreakpointsRequestArguments,
    ) -> Result<SetBreakpointsResponseBody, DapError> {
        Err(DapError::Respond(PartialErrorResponse::new(
            "Unsupported request 'setBreakpoints'".to_string(),
        )))
    }

    async fn stack_trace(
        &mut self,
        _args: StackTraceRequestArguments,
    ) -> Result<StackTraceResponseBody, DapError> {
        Err(DapError::Respond(PartialErrorResponse::new(
            "Unsupported request 'stackTrace'".to_string(),
        )))
    }

    async fn terminate(&mut self, _args: TerminateRequestArguments) -> Result<(), DapError> {
        Err(DapError::Respond(PartialErrorResponse::new(
            "Unsupported request 'terminate'".to_string(),
        )))
    }

    async fn threads(&mut self) -> Result<ThreadsResponseBody, DapError> {
        Err(DapError::Respond(PartialErrorResponse::new(
            "Unsupported request 'threads'".to_string(),
        )))
    }

    async fn variables(
        &mut self,
        _args: VariablesRequestArguments,
    ) -> Result<VariablesResponseBody, DapError> {
        Err(DapError::Respond(PartialErrorResponse::new(
            "Unsupported request 'variables'".to_string(),
        )))
    }
}
