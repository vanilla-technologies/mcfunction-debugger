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

pub mod adapter;
pub mod api;
pub mod codec;
pub mod error;
mod executor;
mod installer;
mod receiver;
mod sender;

use api::{CancelErrorResponse, DebugAdapter, DebugAdapterContext, ProgressContext};
use debug_adapter_protocol::{
    events::{Event, ProgressEndEventBody, ProgressStartEventBody},
    requests::Request,
    responses::{ErrorResponse, Response, SuccessResponse},
    ProtocolMessage, ProtocolMessageContent, SequenceNumber,
};
use error::DebugAdapterError;
use executor::DebugAdapterExecutor;
use futures::{future::Either, FutureExt, Sink, SinkExt, Stream, TryFutureExt};
use log::trace;
use receiver::DebugAdapterReceiver;
use sender::DebugAdapterSender;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
};
use tokio::{
    spawn,
    sync::mpsc::{self, unbounded_channel, UnboundedSender},
    try_join,
};
use uuid::Uuid;

pub async fn run_adapter<D, I, O, E>(
    input: I,
    output: O,
    adapter_factory: impl FnOnce(
        UnboundedSender<Either<ProtocolMessage, <D as DebugAdapter>::Message>>,
    ) -> D,
) -> Result<
    (),
    DebugAdapterError<E, <O as Sink<ProtocolMessage>>::Error, <D as DebugAdapter>::CustomError>,
>
where
    D: DebugAdapter + Send + 'static,
    I: Stream<Item = Result<ProtocolMessage, E>> + Unpin + Send + 'static,
    O: Sink<ProtocolMessage> + Unpin + Send + 'static,
    E: Send + 'static,
    <O as Sink<ProtocolMessage>>::Error: Send + 'static,
    <D as DebugAdapter>::CustomError: Send + 'static,
{
    let (outbox_sender, outbox_receiver) = unbounded_channel();
    let outbox = Outbox { outbox_sender };
    let (inbox_sender, inbox_receiver) = unbounded_channel();
    let (cancel_sender, cancel_receiver) = unbounded_channel();
    let adapter = adapter_factory(inbox_sender.clone());
    let (shutdown_sender, shutdown_receiver) = mpsc::channel(1);

    let cancel_data = Arc::new(Mutex::new(CancelData::new()));
    let receiver = DebugAdapterReceiver {
        inbox_sender,
        outbox: outbox.clone(),
        cancel_data: cancel_data.clone(),
        cancel_sender,
        input,
        shutdown_receiver,
    };

    let executor = DebugAdapterExecutor {
        inbox_receiver,
        outbox,
        cancel_data,
        cancel_receiver,
        adapter,
        shutdown_sender,
    };

    let message_writer = MessageWriter::new(output);
    let sender = DebugAdapterSender {
        message_writer,
        outbox_receiver,
    };

    let receiver = spawn(receiver.run());
    let executor = spawn(executor.run());
    let sender = spawn(sender.run());

    try_join!(
        receiver
            .map(Result::unwrap) // Propagate panic
            .map_err(DebugAdapterError::Input),
        executor
            .map(Result::unwrap) // Propagate panic
            .map_err(DebugAdapterError::Custom),
        sender
            .map(Result::unwrap) // Propagate panic
            .map_err(DebugAdapterError::Output),
    )?;

    Ok(())
}

struct CancelData {
    current_request_id: Option<i32>,
    cancelled_request_ids: HashSet<i32>,
    current_progresses: HashMap<String, UnboundedSender<SequenceNumber>>,
}
impl CancelData {
    fn new() -> Self {
        CancelData {
            current_request_id: None,
            cancelled_request_ids: HashSet::new(),
            current_progresses: HashMap::new(),
        }
    }
}

pub struct DebugAdapterContextImpl {
    outbox: Outbox,
    cancel_data: Arc<Mutex<CancelData>>,
    shutdown: bool,
}
impl DebugAdapterContextImpl {
    fn new(outbox: Outbox, cancel_data: Arc<Mutex<CancelData>>) -> DebugAdapterContextImpl {
        DebugAdapterContextImpl {
            outbox,
            cancel_data,
            shutdown: false,
        }
    }
}
impl DebugAdapterContext for &mut DebugAdapterContextImpl {
    fn fire_event(&mut self, event: impl Into<Event> + Send) {
        let event = event.into();
        self.outbox.send(event);
    }

    fn start_cancellable_progress(
        &mut self,
        title: String,
        message: Option<String>,
    ) -> ProgressContext {
        let progress_id = Uuid::new_v4();
        let (cancel_sender, cancel_receiver) = unbounded_channel();
        {
            let mut cancel_data = self.cancel_data.lock().unwrap();
            cancel_data
                .current_progresses
                .insert(progress_id.to_string(), cancel_sender);
        }

        let event = ProgressStartEventBody::builder()
            .progress_id(progress_id.to_string())
            .title(title)
            .message(message)
            .cancellable(true)
            .build();
        self.fire_event(event);

        let progress_id = progress_id.to_string();
        let outbox = self.outbox.clone();
        ProgressContext::new(progress_id, cancel_receiver, outbox)
    }

    fn end_cancellable_progress(&mut self, progress_id: String, message: Option<String>) {
        {
            let mut cancel_data = self.cancel_data.lock().unwrap();
            cancel_data.current_progresses.remove(&progress_id);
        }
        let event = ProgressEndEventBody::builder()
            .progress_id(progress_id)
            .message(message)
            .build();
        self.fire_event(event);
    }

    fn shutdown(&mut self) {
        trace!("Shutting down executor");
        self.shutdown = true
    }
}

#[derive(Clone)]
struct Outbox {
    outbox_sender: UnboundedSender<ProtocolMessageContent>,
}
impl Outbox {
    fn send(&self, message: impl Into<ProtocolMessageContent>) {
        let _ = self.outbox_sender.send(message.into());
    }

    fn respond(&self, request_id: SequenceNumber, result: Result<SuccessResponse, ErrorResponse>) {
        let response = Response {
            request_seq: request_id,
            result,
        };
        self.send(response);
    }

    fn respond_unknown_progress(&self, request_id: SequenceNumber, progress_id: String) {
        let response = Err(CancelErrorResponse::builder()
            .message(format!("Unknown progress id: {}", progress_id))
            .build()
            .into());
        self.respond(request_id, response);
    }
}

pub struct MessageWriter<O>
where
    O: Sink<ProtocolMessage>,
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
