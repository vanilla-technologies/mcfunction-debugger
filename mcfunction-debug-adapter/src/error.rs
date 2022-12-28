// mcfunction-debugger is a debugger for Minecraft's *.mcfunction files that does not require any
// Minecraft mods.
//
// © Copyright (C) 2021, 2022 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
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
    responses::{ErrorResponse, ErrorResponseBody},
    types::Message as ErrorMessage,
};
use std::io;

#[derive(Debug)]
pub enum DebugAdapterError<I, O, C> {
    Input(I),
    Output(O),
    Custom(C),
}
impl<E> DebugAdapterError<E, E, E> {
    pub fn into_inner(self) -> E {
        match self {
            DebugAdapterError::Input(e) => e,
            DebugAdapterError::Output(e) => e,
            DebugAdapterError::Custom(e) => e,
        }
    }
}

pub enum RequestError<C> {
    Terminate(C),
    Respond(PartialErrorResponse),
}
impl<C> From<PartialErrorResponse> for RequestError<C> {
    fn from(error: PartialErrorResponse) -> Self {
        Self::Respond(error)
    }
}

pub struct PartialErrorResponse {
    pub message: String,
    pub details: Option<ErrorMessage>,
}

impl PartialErrorResponse {
    pub fn new(message: String) -> PartialErrorResponse {
        PartialErrorResponse {
            message,
            details: None,
        }
    }

    pub fn with_command(self, command: String) -> ErrorResponse {
        ErrorResponse::builder()
            .command(command)
            .message(self.message)
            .body(ErrorResponseBody::new(self.details))
            .build()
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
