# McFunction-Debugger is a debugger for Minecraft's *.mcfunction files that does not require any
# Minecraft mods.
#
# © Copyright (C) 2021-2023 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
#
# This file is part of McFunction-Debugger.
#
# McFunction-Debugger is free software: you can redistribute it and/or modify it under the terms of
# the GNU General Public License as published by the Free Software Foundation, either version 3 of
# the License, or (at your option) any later version.
#
# McFunction-Debugger is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
# without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License along with McFunction-Debugger.
# If not, see <http://www.gnu.org/licenses/>.

# -minect_log_conditional-
execute unless score -fn_score_holder- -ns-_valid matches 1 run summon area_effect_cloud ~ ~ ~ {CustomName: '{"text":"error+function_invalid+-orig_ns-:-orig/fn-"}'}
execute unless score -fn_score_holder- -ns-_valid matches 1 run tellraw @a [{"text":""},{"text":"[Error]","color":"red","hoverEvent":{"action":"show_text","contents":"mcfunction-Debugger"}},{"text":" Cannot debug -orig_ns-:-orig/fn-, because it contains an invalid command!"}]
execute unless score -fn_score_holder- -ns-_valid matches 1 run function -ns-:abort_session
execute if score -fn_score_holder- -ns-_valid matches 1 run function -ns-:-orig_ns-/-orig/fn-/start_valid
