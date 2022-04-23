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

use debug_adapter_protocol::{
    requests::Request,
    responses::{ErrorResponse, Response, SuccessResponse},
    ProtocolMessage, ProtocolMessageType, SequenceNumber,
};
use log::trace;
use serde_json::Value;
use std::{collections::HashMap, io};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

const CONTENT_LENGTH: &str = "Content-Length";

pub async fn read_msg<I>(input: &mut I) -> io::Result<ProtocolMessage>
where
    I: AsyncBufReadExt + Unpin,
{
    let header = read_header(input).await?;
    let content_length = get_content_length(&header)?;

    let mut buf = vec![0; content_length];
    input.read_exact(&mut buf).await?;

    let msg = serde_json::from_slice(&buf)?;
    trace!("Received message from client: {}", msg);
    Ok(msg)
}

async fn read_header<I>(input: &mut I) -> io::Result<HashMap<String, String>>
where
    I: AsyncBufReadExt + Unpin,
{
    let mut header = HashMap::new();
    let mut line = String::new();
    loop {
        input.read_line(&mut line).await?;
        if line.ends_with("\r\n") {
            line.pop(); // Pop \n
            line.pop(); // Pop \r
            if line.is_empty() {
                return Ok(header);
            }
            let (key, value) = line.split_once(": ").ok_or_else(|| {
                invalid_data(format!(
                    "Key and value of header field not seperated by a colon and a space: '{}'",
                    line
                ))
            })?;
            header.insert(key.to_string(), value.to_string());
            line.clear();
        }
    }
}

fn get_content_length(header: &HashMap<String, String>) -> io::Result<usize> {
    let content_length = header
        .get(CONTENT_LENGTH)
        .ok_or_else(|| invalid_data("Missing header field 'Content-Length'"))?
        .parse::<usize>()
        .map_err(|_| invalid_data("Header field 'Content-Length' does not have usize value"))?;
    Ok(content_length)
}

pub struct MessageWriter<O>
where
    O: AsyncWriteExt + Unpin,
{
    seq: SequenceNumber,
    output: O,
}

impl<O> MessageWriter<O>
where
    O: AsyncWriteExt + Unpin,
{
    pub fn new(output: O) -> MessageWriter<O> {
        MessageWriter { seq: 0, output }
    }

    pub async fn respond(
        &mut self,
        request_seq: SequenceNumber,
        result: Result<SuccessResponse, ErrorResponse>,
    ) -> io::Result<()> {
        self.write_msg(ProtocolMessageType::Response(Response {
            request_seq,
            result,
        }))
        .await
    }

    pub async fn write_msg(&mut self, msg_type: ProtocolMessageType) -> io::Result<()> {
        self.seq += 1;
        let msg = ProtocolMessage {
            seq: self.seq,
            type_: msg_type,
        };

        let msg = msg.to_string();
        trace!("Sending message to client: {}", msg);
        self.output.write_all(msg.as_bytes()).await?;
        self.output.flush().await?;

        Ok(())
    }
}

fn invalid_data<E>(error: E) -> io::Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    io::Error::new(io::ErrorKind::InvalidData, error)
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
