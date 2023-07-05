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

use crate::dap::{CancelData, Outbox};
use debug_adapter_protocol::{
    requests::{CancelRequestArguments, Request},
    ProtocolMessage, ProtocolMessageContent, SequenceNumber,
};
use futures::{
    future::{select, Either},
    pin_mut, Stream, StreamExt,
};
use log::trace;
use std::sync::{Arc, Mutex};
use tokio::sync::mpsc::{self, UnboundedSender};

pub(super) struct DebugAdapterReceiver<I, E, M>
where
    I: Stream<Item = Result<ProtocolMessage, E>> + Unpin + 'static + Send,
{
    pub inbox_sender: UnboundedSender<Either<ProtocolMessage, M>>,
    pub outbox: Outbox,
    pub cancel_data: Arc<Mutex<CancelData>>,
    pub cancel_sender: UnboundedSender<SequenceNumber>,
    pub input: I,
    pub shutdown_receiver: mpsc::Receiver<()>,
}

impl<I, E, M> DebugAdapterReceiver<I, E, M>
where
    I: Stream<Item = Result<ProtocolMessage, E>> + Unpin + Send + 'static,
{
    pub async fn run(mut self) -> Result<(), E> {
        trace!("Starting receiver");
        while let Some(message) = self.next_input().await {
            let message = message?;
            trace!("Received message from client: {}", message);
            if let ProtocolMessageContent::Request(Request::Cancel(args)) = message.content {
                self.handle_cancel_request(message.seq, args);
            } else {
                if let ProtocolMessageContent::Request(Request::Terminate(_)) = &message.content {
                    self.cancel_all_progresses();
                }
                let _ = self.inbox_sender.send(Either::Left(message));
            }
        }
        trace!("Stopped receiver");
        Ok(())
    }

    async fn next_input(&mut self) -> Option<Result<ProtocolMessage, E>> {
        let shutdown = self.shutdown_receiver.recv();
        pin_mut!(shutdown);
        match select(self.input.next(), shutdown).await {
            Either::Left((result, _)) => result,
            Either::Right(_) => None,
        }
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

    fn cancel_all_progresses(&self) {
        let mut cancel_data = self.cancel_data.lock().unwrap();
        cancel_data.current_progresses.clear();
    }
}
