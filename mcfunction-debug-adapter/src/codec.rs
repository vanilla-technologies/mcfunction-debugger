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

use bytes::{Buf, BytesMut};
use debug_adapter_protocol::ProtocolMessage;
use std::{collections::BTreeMap, io};
use tokio_util::codec::{Decoder, Encoder};

pub struct ProtocolMessageEncoder;
impl Encoder<ProtocolMessage> for ProtocolMessageEncoder {
    type Error = std::io::Error;

    fn encode(&mut self, item: ProtocolMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        const HEADER_PREFIX: &str = "Content-Length: ";
        const HEADER_DELIMITER: &str = "\r\n\r\n";
        let json = serde_json::to_string(&item).unwrap();
        let content_length = json.len().to_string();
        dst.reserve(
            HEADER_PREFIX.len() + content_length.len() + HEADER_DELIMITER.len() + json.len(),
        );
        dst.extend_from_slice(HEADER_PREFIX.as_bytes());
        dst.extend_from_slice(content_length.as_bytes());
        dst.extend_from_slice(HEADER_DELIMITER.as_bytes());
        dst.extend_from_slice(json.as_bytes());
        Ok(())
    }
}

pub struct ProtocolMessageDecoder;
impl Decoder for ProtocolMessageDecoder {
    type Item = ProtocolMessage;
    type Error = io::Error;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        let string = std::str::from_utf8(src).map_err(|e| invalid_data(e))?;
        if let Some((header_len, content_length)) = read_header(string)? {
            let message_len = header_len + content_length;
            if string.len() < message_len {
                Ok(None)
            } else {
                let content = &string[header_len..message_len];
                let message = serde_json::from_str(content)?;
                src.advance(message_len);
                Ok(message)
            }
        } else {
            Ok(None)
        }
    }
}

const CONTENT_LENGTH: &str = "Content-Length";

fn read_header(string: &str) -> Result<Option<(usize, usize)>, io::Error> {
    const HEADER_DELIMITER: &str = "\r\n\r\n";
    let header_end = if let Some(header_end) = string.find(HEADER_DELIMITER) {
        header_end
    } else {
        return Ok(None);
    };
    let mut header = BTreeMap::new();

    for line in string[..header_end].split("\r\n") {
        let (key, value) = line.split_once(": ").ok_or_else(|| {
            invalid_data(format!(
                "Key and value of header field not seperated by a colon and a space: '{}'",
                line
            ))
        })?;
        header.insert(key, value);
    }
    let content_length = get_content_length(&header)?;
    Ok(Some((header_end + HEADER_DELIMITER.len(), content_length)))
}

fn get_content_length(header: &BTreeMap<&str, &str>) -> io::Result<usize> {
    let content_length = &header
        .get(CONTENT_LENGTH)
        .ok_or_else(|| invalid_data(format!("Missing header '{}'", CONTENT_LENGTH)))?;
    let content_length = content_length.parse().map_err(|_| {
        invalid_data(format!(
            "Header '{}' does not have usize value: {}",
            CONTENT_LENGTH, content_length
        ))
    })?;
    Ok(content_length)
}

fn invalid_data<E>(error: E) -> io::Error
where
    E: Into<Box<dyn std::error::Error + Send + Sync>>,
{
    io::Error::new(io::ErrorKind::InvalidData, error)
}
