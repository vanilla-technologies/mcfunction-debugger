// mcfunction-debugger is a debugger for Minecraft's *.mcfunction files that does not require any
// Minecraft mods.
//
// © Copyright (C) 2021-2023 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
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

use futures::{FutureExt, Stream};
use std::time::Duration;
use tokio::time::timeout;
use tokio_stream::StreamExt;

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

pub struct TimeoutStream<S, I>
where
    S: Stream<Item = I> + Unpin,
{
    inner: S,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TimeoutStreamError {
    Timeout,
    Closed,
}

impl<S, I> TimeoutStream<S, I>
where
    S: Stream<Item = I> + Unpin,
{
    pub fn new(inner: S) -> Self {
        TimeoutStream { inner }
    }

    pub async fn next(&mut self) -> Result<I, TimeoutStreamError> {
        self.next_timeout(DEFAULT_TIMEOUT).await
    }

    pub async fn next_timeout(&mut self, duration: Duration) -> Result<I, TimeoutStreamError> {
        timeout(duration, self.inner.next())
            .await
            .map(|it| it.ok_or(TimeoutStreamError::Closed))
            .map_err(|_| TimeoutStreamError::Timeout)
            .and_then(|it| it)
    }

    pub fn try_next(&mut self) -> Result<I, TimeoutStreamError> {
        self.inner
            .next()
            .now_or_never()
            .map(|it| it.ok_or(TimeoutStreamError::Closed))
            .ok_or(TimeoutStreamError::Timeout)
            .and_then(|it| it)
    }
}
