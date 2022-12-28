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

use crate::MessageWriter;
use debug_adapter_protocol::{ProtocolMessage, ProtocolMessageContent};
use futures::Sink;
use tokio::sync::mpsc::UnboundedReceiver;

pub(super) struct DebugAdapterSender<O>
where
    O: Sink<ProtocolMessage>,
{
    pub message_writer: MessageWriter<O>,
    pub outbox_receiver: UnboundedReceiver<ProtocolMessageContent>,
}

impl<O> DebugAdapterSender<O>
where
    O: Sink<ProtocolMessage> + Unpin,
{
    pub async fn run(&mut self) -> Result<(), O::Error> {
        while let Some(message) = self.outbox_receiver.recv().await {
            self.message_writer.write_msg(message).await?;
        }
        Ok(())
    }
}
