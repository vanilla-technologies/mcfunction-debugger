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
    responses::{ErrorResponse, Response, SuccessResponse},
    ProtocolMessage, ProtocolMessageType, SequenceNumber,
};
use std::{collections::HashMap, io};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt};

const CONTENT_LENGTH: &str = "Content-Length";

pub async fn read_msg<I, L>(input: &mut I, log: &mut L) -> io::Result<ProtocolMessage>
where
    I: AsyncBufReadExt + Unpin,
    L: AsyncWriteExt + Unpin,
{
    let header = read_header(input, log).await?;
    let content_length = get_content_length(&header)?;

    let mut buf = vec![0; content_length];
    input.read_exact(&mut buf).await?;
    log.write_all(&buf).await?;
    log.flush().await?;

    let msg = serde_json::from_slice(&buf)?;
    Ok(msg)
}

async fn read_header<I, L>(input: &mut I, log: &mut L) -> io::Result<HashMap<String, String>>
where
    I: AsyncBufReadExt + Unpin,
    L: AsyncWriteExt + Unpin,
{
    let mut header = HashMap::new();
    let mut line = String::new();
    loop {
        input.read_line(&mut line).await?;
        if line.ends_with("\r\n") {
            log.write_all(line.as_bytes()).await?;
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

pub struct MessageWriter<O, L>
where
    O: AsyncWriteExt + Unpin,
    L: AsyncWriteExt + Unpin,
{
    seq: SequenceNumber,
    output: O,
    log: L,
}

impl<O, L> MessageWriter<O, L>
where
    O: AsyncWriteExt + Unpin,
    L: AsyncWriteExt + Unpin,
{
    pub fn new(output: O, log: L) -> MessageWriter<O, L> {
        MessageWriter {
            seq: 0,
            output,
            log,
        }
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
        let json = serde_json::to_string(&msg).unwrap();

        self.output.write_all("Content-Length: ".as_bytes()).await?;
        self.log.write_all("Content-Length: ".as_bytes()).await?;

        self.output
            .write_all(json.len().to_string().as_bytes())
            .await?;
        self.log
            .write_all(json.len().to_string().as_bytes())
            .await?;

        self.output.write_all("\r\n\r\n".as_bytes()).await?;
        self.log.write_all("\r\n\r\n".as_bytes()).await?;

        self.output.write_all(json.as_bytes()).await?;
        self.log.write_all(json.as_bytes()).await?;

        self.output.flush().await?;
        self.log.flush().await?;

        Ok(())
    }
}

fn invalid_data<E>(error: E) -> io::Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    io::Error::new(io::ErrorKind::InvalidData, error)
}
