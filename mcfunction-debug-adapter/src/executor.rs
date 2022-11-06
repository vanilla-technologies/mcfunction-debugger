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

use crate::{
    api::DebugAdapter, error::RequestError, get_command, CancelData, DebugAdapterContextImpl,
    Outbox,
};
use debug_adapter_protocol::{
    responses::{ErrorResponse, ErrorResponseBody, SuccessResponse},
    ProtocolMessage, ProtocolMessageContent, SequenceNumber,
};
use futures::{
    future::{select, Either},
    pin_mut,
};
use log::trace;
use std::{io, sync::Mutex};
use tokio::sync::mpsc::{error::TryRecvError, UnboundedReceiver};

pub(super) struct DebugAdapterExecutor<'l, D>
where
    D: DebugAdapter,
{
    pub cancel_data: &'l Mutex<CancelData>,
    pub inbox_receiver: UnboundedReceiver<Either<ProtocolMessage, <D as DebugAdapter>::Message>>,
    pub outbox: Outbox,
    pub cancel_receiver: UnboundedReceiver<SequenceNumber>,
    pub adapter: D,
}

impl<D> DebugAdapterExecutor<'_, D>
where
    D: DebugAdapter + Send,
{
    pub async fn run(&mut self) -> Result<(), <D as DebugAdapter>::CustomError> {
        while let Some(msg) = self.inbox_receiver.recv().await {
            match msg {
                Either::Left(client_msg) => {
                    trace!("Handling message from client: {}", client_msg);

                    let seq = client_msg.seq; // TODO: seq zu i32 machen
                    let mut maybe_cancel_request_id = None;
                    // TODO: ugly
                    let command =
                        if let ProtocolMessageContent::Request(request) = &client_msg.content {
                            get_command(request)
                        } else {
                            "".to_string()
                        };
                    if self.start_request(seq as i32) {
                        {
                            let cancel = self.cancel_receiver.recv();
                            pin_mut!(cancel);
                            let context = DebugAdapterContextImpl {
                                outbox: &mut self.outbox,
                                cancel_data: self.cancel_data,
                            };
                            let handle_message =
                                handle_client_message(client_msg, &mut self.adapter, context);
                            pin_mut!(handle_message);

                            match select(cancel, handle_message).await {
                                Either::Left((Some(cancel_request_id), _)) => {
                                    maybe_cancel_request_id = Some(cancel_request_id);
                                }
                                Either::Left((None, _)) => {
                                    // TODO panic
                                    panic!("cancel channel was closed");
                                    // return Err(io::Error::new(
                                    //     io::ErrorKind::BrokenPipe,
                                    //     "cancel channel was closed",
                                    // ))
                                }
                                Either::Right((result, _)) => {
                                    let should_continue = result?;
                                    if !should_continue {
                                        break;
                                    }
                                }
                            }
                        }
                        if let Some(cancel_request_id) = maybe_cancel_request_id {
                            let response = Err(ErrorResponse::builder()
                                .command(command)
                                .message("cancelled".to_string())
                                .body(ErrorResponseBody::new(None))
                                .build());
                            self.outbox.respond(seq, response);

                            self.outbox
                                .respond(cancel_request_id, Ok(SuccessResponse::Cancel));
                        }
                        // TODO panic
                        self.finish_request().unwrap();
                    }
                }
                Either::Right(message) => {
                    let mut context = DebugAdapterContextImpl {
                        outbox: &mut self.outbox,
                        cancel_data: self.cancel_data,
                    };
                    let should_continue = self
                        .adapter
                        .handle_other_message(message, &mut context)
                        .await?;
                    if !should_continue {
                        break;
                    }
                }
            }
        }
        Ok(())
    }

    fn start_request(&self, request_id: i32) -> bool {
        let mut cancel_data = self.cancel_data.lock().unwrap();
        let is_cancelled = cancel_data.cancelled_request_ids.remove(&request_id);
        if !is_cancelled {
            cancel_data.current_request_id = Some(request_id);
        }
        !is_cancelled
    }

    fn finish_request(&mut self) -> io::Result<()> {
        let mut cancel_data = self.cancel_data.lock().unwrap();
        cancel_data.current_request_id = None;
        clear_channel(&mut self.cancel_receiver)?; // Clear all remaining cancel requests
        Ok(())
    }
}

async fn handle_client_message<D>(
    msg: ProtocolMessage,
    adapter: &mut D,
    mut context: DebugAdapterContextImpl<'_>,
) -> Result<bool, <D as DebugAdapter>::CustomError>
where
    D: DebugAdapter + Send,
{
    match msg.content {
        ProtocolMessageContent::Request(request) => {
            let command = get_command(&request);

            let result = adapter.handle_client_request(request, &mut context).await;

            let mut should_continue = true;
            let response = match result {
                Ok(response) => {
                    if response == SuccessResponse::Disconnect {
                        should_continue = true;
                    }
                    Ok(response)
                }
                Err(RequestError::Respond(response)) => Err(response.with_command(command)),
                Err(RequestError::Terminate(e)) => return Err(e),
            };
            context.outbox.respond(msg.seq, response);
            Ok(should_continue)
        }
        _ => {
            todo!("Only requests and RunInTerminalResponse should be sent by the client");
        }
    }
}

fn clear_channel<E>(receiver: &mut UnboundedReceiver<E>) -> io::Result<()> {
    loop {
        match receiver.try_recv() {
            Ok(_) => {}
            Err(TryRecvError::Empty) => {
                return Ok(());
            }
            Err(TryRecvError::Disconnected) => {
                return Err(io::Error::new(
                    io::ErrorKind::BrokenPipe,
                    TryRecvError::Disconnected,
                ))
            }
        }
    }
}
