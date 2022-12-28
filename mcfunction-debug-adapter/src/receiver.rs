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

use crate::{CancelData, Outbox};
use debug_adapter_protocol::{
    requests::{CancelRequestArguments, Request},
    ProtocolMessage, ProtocolMessageContent, SequenceNumber,
};
use futures::{future::Either, Stream, StreamExt};
use log::trace;
use std::{io, sync::Mutex};
use tokio::sync::mpsc::UnboundedSender;

pub(super) struct DebugAdapterReceiver<'l, I, M>
where
    I: Stream<Item = io::Result<ProtocolMessage>> + Unpin + 'static + Send,
{
    pub inbox_sender: UnboundedSender<Either<ProtocolMessage, M>>,
    pub outbox: Outbox,
    pub cancel_data: &'l Mutex<CancelData>,
    pub cancel_sender: UnboundedSender<SequenceNumber>,
    pub input: I,
}

impl<I, M> DebugAdapterReceiver<'_, I, M>
where
    I: Stream<Item = io::Result<ProtocolMessage>> + Unpin + Send + 'static,
{
    pub async fn run(&mut self) -> Result<(), io::Error> {
        while let Some(message) = self.input.next().await {
            let message = message?;
            trace!("Received message from client: {}", message);
            if let ProtocolMessageContent::Request(Request::Cancel(args)) = message.content {
                self.handle_cancel_request(message.seq, args);
            } else {
                if let ProtocolMessageContent::Request(Request::Terminate(_)) = &message.content {
                    self.handle_terminate_request();
                }
                let _ = self.inbox_sender.send(Either::Left(message));
            }
        }
        Ok(())
    }

    fn handle_cancel_request(
        &self,
        cancel_request_id: SequenceNumber,
        args: CancelRequestArguments,
    ) {
        let mut cancel_data = self.cancel_data.lock().unwrap();

        if let Some(progress_id) = args.progress_id {
            if let Some(cancel_sender) = cancel_data.current_progresses.get_mut(&progress_id) {
                let _ = cancel_sender.send(cancel_request_id);
            } else {
                self.outbox
                    .respond_unknown_progress(cancel_request_id, progress_id);
            }
        }

        let cancel_current_request_id =
            args.request_id.is_some() && args.request_id == cancel_data.current_request_id;
        if cancel_current_request_id {
            let _ = self.cancel_sender.send(cancel_request_id);
        } else {
            if let Some(request_id) = args.request_id {
                // TODO: memory leak: better only insert request_ids that are currently in queue
                cancel_data.cancelled_request_ids.insert(request_id);
            }
        }
    }

    fn handle_terminate_request(&self) {
        let mut cancel_data = self.cancel_data.lock().unwrap();
        cancel_data.current_progresses.clear();

        // TODO: cancel all queued requests
    }
}
