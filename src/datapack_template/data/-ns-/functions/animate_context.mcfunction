# mcfunction-debugger is a debugger for Minecraft's *.mcfunction files that does not require any
# Minecraft mods.
#
# © Copyright (C) 2021-2023 Adrodoc <adrodoc55@googlemail.com> & skess42 <skagaros@gmail.com>
#
# This file is part of mcfunction-debugger.
#
# mcfunction-debugger is free software: you can redistribute it and/or modify it under the terms of
# the GNU General Public License as published by the Free Software Foundation, either version 3 of
# the License, or (at your option) any later version.
#
# mcfunction-debugger is distributed in the hope that it will be useful, but WITHOUT ANY WARRANTY;
# without even the implied warranty of MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE. See the
# GNU General Public License for more details.
#
# You should have received a copy of the GNU General Public License along with mcfunction-debugger.
# If not, see <http://www.gnu.org/licenses/>.

# position
particle composter ~ ~ ~ 0 0 0 0 1

# rotation
scoreboard players add rotation_animation_frame -ns-_global 1
scoreboard players operation rotation_animation_frame -ns-_global %= 88 -ns-_constant
execute if score rotation_animation_frame -ns-_global matches 0 positioned ~ ~.15 ~ run particle underwater ^ ^ ^.1 0 0 0 0 80
execute if score rotation_animation_frame -ns-_global matches 2 positioned ~ ~.15 ~ run particle underwater ^ ^ ^.2 0 0 0 0 80
execute if score rotation_animation_frame -ns-_global matches 4 positioned ~ ~.15 ~ run particle underwater ^ ^ ^.3 0 0 0 0 80
execute if score rotation_animation_frame -ns-_global matches 6 positioned ~ ~.15 ~ run particle underwater ^ ^ ^.4 0 0 0 0 80
execute if score rotation_animation_frame -ns-_global matches 8 positioned ~ ~.15 ~ run particle underwater ^ ^ ^.5 0 0 0 0 80
execute if score rotation_animation_frame -ns-_global matches 10 positioned ~ ~.15 ~ run particle underwater ^ ^ ^.6 0 0 0 0 80
execute if score rotation_animation_frame -ns-_global matches 12 positioned ~ ~.15 ~ run particle underwater ^ ^ ^.7 0 0 0 0 80
execute if score rotation_animation_frame -ns-_global matches 14 positioned ~ ~.15 ~ run particle underwater ^ ^ ^.8 0 0 0 0 80
execute if score rotation_animation_frame -ns-_global matches 16 positioned ~ ~.15 ~ run particle underwater ^ ^ ^.9 0 0 0 0 80
execute if score rotation_animation_frame -ns-_global matches 18 positioned ~ ~.15 ~ run particle underwater ^ ^ ^1 0 0 0 0 80
