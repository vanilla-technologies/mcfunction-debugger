execute unless score breakpoint -ns-_global matches 1 store success score schedule_success -ns-_global run kill @e[type=area_effect_cloud,tag=schedule_-orig_ns-_-orig_fn-,nbt={Age: -1}]
execute unless score breakpoint -ns-_global matches 1 if score schedule_success -ns-_global matches 1 run function -ns-:-orig_ns-/-orig/fn-/start
