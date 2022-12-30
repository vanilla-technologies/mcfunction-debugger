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

use minect::log::{AddTagOutput, QueryScoreboardOutput};

pub fn parse_scoreboard_value(output: &str, scoreboard: &str) -> Option<i32> {
    let output = output.parse::<QueryScoreboardOutput>().ok()?;
    if output.scoreboard == scoreboard {
        Some(output.score)
    } else {
        None
    }
}

pub fn is_added_tag_output(output: &str, tag: &str) -> bool {
    if let Ok(output) = output.parse::<AddTagOutput>() {
        output.tag == tag
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_is_added_tag_output_correct_tag() {
        // given:
        let output = "Added tag 'my_tag' to my_entity";
        let tag = "my_tag";

        // when:
        let actual = is_added_tag_output(output, tag);

        // then:
        assert_eq!(actual, true);
    }

    #[test]
    fn test_is_added_tag_output_wrong_tag() {
        // given:
        let output = "Added tag 'not_my_tag' to my_entity";
        let tag = "my_tag";

        // when:
        let actual = is_added_tag_output(output, tag);

        // then:
        assert_eq!(actual, false);
    }

    #[test]
    fn test_is_added_tag_output_wrong_output() {
        // given:
        let output = "Bla bla bla";
        let tag = "my_tag";

        // when:
        let actual = is_added_tag_output(output, tag);

        // then:
        assert_eq!(actual, false);
    }
}
