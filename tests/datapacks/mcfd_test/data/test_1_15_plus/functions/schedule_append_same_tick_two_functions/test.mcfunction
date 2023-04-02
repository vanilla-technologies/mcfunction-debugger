scoreboard players set test_score test_global 0
schedule function test:utils/add_1 1t append
schedule function test:utils/add_2 1t append
schedule function test_1_15_plus:schedule_append_same_tick_two_functions/assert 2t
