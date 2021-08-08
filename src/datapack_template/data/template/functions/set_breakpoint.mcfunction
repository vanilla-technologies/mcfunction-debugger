tellraw @a [{"text": "Suspended at breakpoint -orig_ns-:-orig/fn-:-line_number-\nTo resume run: ","color": "gold"},{"text": "/function debug:resume","clickEvent": {"action": "suggest_command","value": "/function debug:resume"},"color": "aqua"}]
scoreboard players set breakpoint -ns-_global 1
summon area_effect_cloud ~ ~ ~ {Age: -2147483648, Duration: -1, WaitTime: -2147483648, CustomName: '{"text":"at -orig_ns-:-orig/fn-:-line_number-"}', Tags: [-ns-, -ns-_breakpoint, -ns-_-orig_ns-_-orig_fn-_-line_number-]}
execute as @e[type=area_effect_cloud] run function -ns-:freeze_aec
