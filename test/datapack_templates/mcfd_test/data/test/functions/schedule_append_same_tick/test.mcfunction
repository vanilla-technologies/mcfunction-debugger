scoreboard players set schedule_append test_global 0
schedule function test:schedule_append_same_tick/increment 1t replace
schedule function test:schedule_append_same_tick/increment 1t append
schedule function test:schedule_append_same_tick/assert 2t
