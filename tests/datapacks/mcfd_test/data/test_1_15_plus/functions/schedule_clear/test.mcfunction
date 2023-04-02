scoreboard players set test_score test_global 0
schedule function test_1_15_plus:schedule_clear/increment 1t
schedule clear test_1_15_plus:schedule_clear/increment
schedule function test_1_15_plus:schedule_clear/assert 2t
