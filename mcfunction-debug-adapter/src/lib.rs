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

use debug_adapter_protocol::{
    requests::Request,
    responses::{ErrorResponse, Response, SuccessResponse},
    ProtocolMessage, ProtocolMessageContent, SequenceNumber,
};
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
