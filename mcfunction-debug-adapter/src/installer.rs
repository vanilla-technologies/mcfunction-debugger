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

use crate::{
    api::ProgressContext,
    error::{PartialErrorResponse, RequestError},
    DebugAdapterContext,
};
use futures::{
    future::{select, Either},
    pin_mut,
};
use minect::MinecraftConnection;
use std::{io, path::Path};

pub async fn establish_connection(
    minecraft_world_dir: impl AsRef<Path>,
    minecraft_log_file: impl AsRef<Path>,
    mut context: impl DebugAdapterContext,
) -> Result<MinecraftConnection, RequestError<io::Error>> {
    let mut progress = context.start_cancellable_progress(
        "Connecting to Minecraft".to_string(),
        Some(
            "If you are connecting for the first time please execute /reload in Minecraft."
                .to_string(),
        ),
    );

    let mut connection =
        MinecraftConnection::builder("mcfunction-debugger", minecraft_world_dir.as_ref())
            .log_file(minecraft_log_file.as_ref())
            .build();
    let result = connect(&mut connection, &mut progress).await;

    let progress_id = progress.progress_id.to_string();
    let progress_end_message = match &result {
        Ok(()) => "Successfully connected to Minecraft".to_string(),
        Err(ConnectError::Cancelled) => "Cancelled connecting to Minecraft".to_string(),
        Err(ConnectError::Failed(error)) => format!("Failed to connect to Minecraft: {}", error),
    };
    context.end_cancellable_progress(progress_id, Some(progress_end_message));

    result
        .map_err(|e| match e {
            ConnectError::Cancelled => "Launch was cancelled.".to_string(),
            ConnectError::Failed(error) => format!("Failed to connect to Minecraft: {}", error),
        })
        .map_err(PartialErrorResponse::new)?;

    Ok(connection)
}

enum ConnectError {
    Cancelled,
    Failed(minect::ConnectError),
}
impl From<minect::ConnectError> for ConnectError {
    fn from(error: minect::ConnectError) -> Self {
        if error.is_cancelled() {
            ConnectError::Cancelled
        } else {
            ConnectError::Failed(error)
        }
    }
}
async fn connect(
    connection: &mut MinecraftConnection,
    progress: &mut ProgressContext,
) -> Result<(), ConnectError> {
    let connect = connection.connect();
    pin_mut!(connect);
    let cancel = progress.next_cancel_request();
    pin_mut!(cancel);
    match select(connect, cancel).await {
        Either::Left((result, _)) => {
            result?;
            Ok(())
        }
        Either::Right(_) => return Err(ConnectError::Cancelled),
    }
}
