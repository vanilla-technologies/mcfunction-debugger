scoreboard players set append test 0
schedule function test:schedule_append_same_tick/count_append 1t replace
schedule function test:schedule_append_same_tick/count_append 1t append
schedule function test:schedule_append_same_tick/test_append_count 2t
