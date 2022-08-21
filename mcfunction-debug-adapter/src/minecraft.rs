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

/// Parse an event in the following format:
///
/// `[15:58:32] [Server thread/INFO]: [sample:main:2: Added 0 to [mcfd_depth] for 22466a74-94bd-458b-af97-3333c36d7b0b (now 1)]`
pub fn parse_scoreboard_value(message: &str, scoreboard: &str) -> Option<i32> {
    let suffix = message.strip_prefix(&format!("Added 0 to [{}] for ", scoreboard))?;
    const NOW: &str = " (now ";
    let index = suffix.find(NOW)?;
    let suffix = &suffix[index + NOW.len()..];
    let scoreboard_value = suffix.strip_suffix(')')?;
    scoreboard_value.parse().ok()
}

pub struct ScoreboardMessage {
    pub scoreboard: String,
    pub entity: String,
    pub score: i32,
}
impl ScoreboardMessage {
    pub fn parse(message: &str) -> Option<ScoreboardMessage> {
        let suffix = message.strip_prefix(&format!("Added 0 to ["))?;
        const FOR: &str = "] for ";
        let index = suffix.find(FOR)?;
        let (scoreboard, suffix) = suffix.split_at(index);
        let suffix = suffix.strip_prefix(FOR)?;

        const NOW: &str = " (now ";
        let index = suffix.find(NOW)?;
        let (entity, suffix) = suffix.split_at(index);
        let suffix = suffix.strip_prefix(NOW)?;
        let score = suffix.strip_suffix(')')?;
        let score = score.parse().ok()?;

        Some(ScoreboardMessage {
            scoreboard: scoreboard.to_string(),
            entity: entity.to_string(),
            score,
        })
    }
}

/// Parse an event in the following format:
///
/// `[16:09:59] [Server thread/INFO]: [sample:foo:2: Added tag 'mcfd_breakpoint' to sample:foo:2]`
pub fn parse_added_tag_message(message: &str) -> Option<&str> {
    let suffix = message.strip_prefix("Added tag '")?;
    const TO: &str = "' to ";
    let index = suffix.find(TO)?;
    Some(&suffix[..index])
}

pub fn is_added_tag_message(message: &str, tag: &str) -> bool {
    if let Some(actual_tag) = parse_added_tag_message(message) {
        actual_tag == tag
    } else {
        false
    }
}
