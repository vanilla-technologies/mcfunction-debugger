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

use crate::{
    error::{PartialErrorResponse, RequestError},
    get_command, Outbox,
};
use async_trait::async_trait;
use debug_adapter_protocol::{
    events::Event,
    requests::{
        ContinueRequestArguments, DisconnectRequestArguments, EvaluateRequestArguments,
        InitializeRequestArguments, LaunchRequestArguments, NextRequestArguments,
        PauseRequestArguments, Request, ScopesRequestArguments, SetBreakpointsRequestArguments,
        StackTraceRequestArguments, StepInRequestArguments, StepOutRequestArguments,
        TerminateRequestArguments, VariablesRequestArguments,
    },
    responses::{
        ContinueResponseBody, ErrorResponse, ErrorResponseBody, EvaluateResponseBody,
        ScopesResponseBody, SetBreakpointsResponseBody, StackTraceResponseBody, SuccessResponse,
        ThreadsResponseBody, VariablesResponseBody,
    },
    types::Capabilities,
    SequenceNumber,
};
use tokio::sync::mpsc::UnboundedReceiver;
use typed_builder::TypedBuilder;

pub trait DebugAdapterContext {
    fn fire_event(&mut self, event: impl Into<Event> + Send);

    fn start_cancellable_progress(
        &mut self,
        title: String,
        message: Option<String>,
    ) -> ProgressContext;

    fn end_cancellable_progress(&mut self, progress_id: String, message: Option<String>);

    fn shutdown(&mut self);
}

pub struct ProgressContext {
    pub progress_id: String,
    cancel_receiver: UnboundedReceiver<SequenceNumber>,
    outbox: Outbox,
}
impl ProgressContext {
    pub(super) fn new(
        progress_id: String,
        cancel_receiver: UnboundedReceiver<SequenceNumber>,
        outbox: Outbox,
    ) -> ProgressContext {
        ProgressContext {
            progress_id,
            cancel_receiver,
            outbox,
        }
    }

    pub async fn next_cancel_request(&mut self) -> Option<CancelRequest> {
        let request_id = self.cancel_receiver.recv().await?;
        Some(CancelRequest {
            outbox: self.outbox.clone(),
            request_id,
            response_sent: false,
        })
    }
}
impl Drop for ProgressContext {
    fn drop(&mut self) {
        while let Ok(open_request) = self.cancel_receiver.try_recv() {
            self.outbox
                .respond_unknown_progress(open_request, self.progress_id.to_string())
        }
    }
}

pub struct CancelRequest {
    outbox: Outbox,
    request_id: SequenceNumber,
    response_sent: bool,
}
impl CancelRequest {
    pub fn respond(mut self, response: Result<(), CancelErrorResponse>) {
        self.respond_without_consuming(response)
    }

    fn respond_without_consuming(&mut self, response: Result<(), CancelErrorResponse>) {
        // Prevent sending response in drop if it was already sent manually
        if self.response_sent {
            return;
        }
        self.response_sent = true;
        let result = response
            .map(|()| SuccessResponse::Cancel)
            .map_err(Into::into);
        self.outbox.respond(self.request_id, result);
    }
}
impl Drop for CancelRequest {
    fn drop(&mut self) {
        self.respond_without_consuming(Ok(()))
    }
}

#[derive(Clone, Debug, Eq, PartialEq, TypedBuilder)]
pub struct CancelErrorResponse {
    /// Contains the raw error in short form if 'success' is false.
    /// This raw error might be interpreted by the frontend and is not shown in the
    /// UI.
    /// Some predefined values exist.
    /// Values:
    /// 'cancelled': request was cancelled.
    /// etc.
    pub message: String,

    #[builder(default)]
    pub body: ErrorResponseBody,

    #[builder(default, setter(skip))]
    private: (),
}
impl From<CancelErrorResponse> for ErrorResponse {
    fn from(value: CancelErrorResponse) -> Self {
        ErrorResponse::builder()
            .command("cancel".to_string())
            .message(value.message)
            .body(value.body)
            .build()
    }
}

#[async_trait]
pub trait DebugAdapter {
    type Message: Send + 'static;
    type CustomError;

    fn map_custom_error(e: Self::CustomError) -> RequestError<Self::CustomError> {
        RequestError::Terminate(e)
    }

    async fn handle_other_message(
        &mut self,
        _message: Self::Message,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<(), Self::CustomError> {
        Ok(())
    }

    async fn handle_client_request(
        &mut self,
        request: Request,
        context: impl DebugAdapterContext + Send,
    ) -> Result<SuccessResponse, RequestError<Self::CustomError>> {
        match request {
            Request::ConfigurationDone => self
                .configuration_done(context)
                .await
                .map(|()| SuccessResponse::ConfigurationDone),
            Request::Continue(args) => self
                .continue_(args, context)
                .await
                .map(SuccessResponse::Continue),
            Request::Disconnect(args) => self
                .disconnect(args, context)
                .await
                .map(|()| SuccessResponse::Disconnect),
            Request::Evaluate(args) => self
                .evaluate(args, context)
                .await
                .map(SuccessResponse::Evaluate),
            Request::Initialize(args) => self
                .initialize(args, context)
                .await
                .map(SuccessResponse::Initialize),
            Request::Launch(args) => self
                .launch(args, context)
                .await
                .map(|()| SuccessResponse::Launch),
            Request::Next(args) => self
                .next(args, context)
                .await
                .map(|()| SuccessResponse::Next),
            Request::Pause(args) => self
                .pause(args, context)
                .await
                .map(|()| SuccessResponse::Pause),
            Request::Scopes(args) => self
                .scopes(args, context)
                .await
                .map(SuccessResponse::Scopes),
            Request::SetBreakpoints(args) => self
                .set_breakpoints(args, context)
                .await
                .map(SuccessResponse::SetBreakpoints),
            Request::StackTrace(args) => self
                .stack_trace(args, context)
                .await
                .map(SuccessResponse::StackTrace),
            Request::StepIn(args) => self
                .step_in(args, context)
                .await
                .map(|()| SuccessResponse::StepIn),
            Request::StepOut(args) => self
                .step_out(args, context)
                .await
                .map(|()| SuccessResponse::StepOut),
            Request::Terminate(args) => self
                .terminate(args, context)
                .await
                .map(|()| SuccessResponse::Terminate),
            Request::Threads => self.threads(context).await.map(SuccessResponse::Threads),
            Request::Variables(args) => self
                .variables(args, context)
                .await
                .map(SuccessResponse::Variables),
            _ => {
                let command = get_command(&request);
                Err(RequestError::Respond(PartialErrorResponse::new(format!(
                    "Unsupported request {}",
                    command
                ))))
            }
        }
    }

    async fn configuration_done(
        &mut self,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<(), RequestError<Self::CustomError>> {
        Err(RequestError::Respond(PartialErrorResponse::new(
            "Unsupported request 'configurationDone'".to_string(),
        )))
    }

    async fn continue_(
        &mut self,
        _args: ContinueRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<ContinueResponseBody, RequestError<Self::CustomError>>;

    async fn disconnect(
        &mut self,
        _args: DisconnectRequestArguments,
        mut context: impl DebugAdapterContext + Send,
    ) -> Result<(), RequestError<Self::CustomError>> {
        context.shutdown();
        Ok(())
    }

    async fn evaluate(
        &mut self,
        _args: EvaluateRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<EvaluateResponseBody, RequestError<Self::CustomError>> {
        Err(RequestError::Respond(PartialErrorResponse::new(
            "Unsupported request 'evaluate'".to_string(),
        )))
    }

    async fn initialize(
        &mut self,
        _args: InitializeRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<Capabilities, RequestError<Self::CustomError>> {
        Err(RequestError::Respond(PartialErrorResponse::new(
            "Unsupported request 'initialize'".to_string(),
        )))
    }

    async fn launch(
        &mut self,
        _args: LaunchRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<(), RequestError<Self::CustomError>> {
        Err(RequestError::Respond(PartialErrorResponse::new(
            "Unsupported request 'launch'".to_string(),
        )))
    }

    async fn next(
        &mut self,
        _args: NextRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<(), RequestError<Self::CustomError>> {
        Err(RequestError::Respond(PartialErrorResponse::new(
            "Unsupported request 'next'".to_string(),
        )))
    }

    async fn pause(
        &mut self,
        _args: PauseRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<(), RequestError<Self::CustomError>> {
        Err(RequestError::Respond(PartialErrorResponse::new(
            "Unsupported request 'pause'".to_string(),
        )))
    }

    async fn scopes(
        &mut self,
        _args: ScopesRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<ScopesResponseBody, RequestError<Self::CustomError>> {
        Err(RequestError::Respond(PartialErrorResponse::new(
            "Unsupported request 'scopes'".to_string(),
        )))
    }

    async fn set_breakpoints(
        &mut self,
        _args: SetBreakpointsRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<SetBreakpointsResponseBody, RequestError<Self::CustomError>> {
        Err(RequestError::Respond(PartialErrorResponse::new(
            "Unsupported request 'setBreakpoints'".to_string(),
        )))
    }

    async fn stack_trace(
        &mut self,
        _args: StackTraceRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<StackTraceResponseBody, RequestError<Self::CustomError>> {
        Err(RequestError::Respond(PartialErrorResponse::new(
            "Unsupported request 'stackTrace'".to_string(),
        )))
    }

    async fn step_in(
        &mut self,
        _args: StepInRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<(), RequestError<Self::CustomError>> {
        Err(RequestError::Respond(PartialErrorResponse::new(
            "Unsupported request 'stepIn'".to_string(),
        )))
    }

    async fn step_out(
        &mut self,
        _args: StepOutRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<(), RequestError<Self::CustomError>> {
        Err(RequestError::Respond(PartialErrorResponse::new(
            "Unsupported request 'stepOut'".to_string(),
        )))
    }

    async fn terminate(
        &mut self,
        _args: TerminateRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<(), RequestError<Self::CustomError>> {
        Err(RequestError::Respond(PartialErrorResponse::new(
            "Unsupported request 'terminate'".to_string(),
        )))
    }

    async fn threads(
        &mut self,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<ThreadsResponseBody, RequestError<Self::CustomError>> {
        Err(RequestError::Respond(PartialErrorResponse::new(
            "Unsupported request 'threads'".to_string(),
        )))
    }

    async fn variables(
        &mut self,
        _args: VariablesRequestArguments,
        _context: impl DebugAdapterContext + Send,
    ) -> Result<VariablesResponseBody, RequestError<Self::CustomError>> {
        Err(RequestError::Respond(PartialErrorResponse::new(
            "Unsupported request 'variables'".to_string(),
        )))
    }
}
