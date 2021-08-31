scoreboard players set test_score test_global 0
schedule function test:schedule_clear/increment 1t
schedule clear test:schedule_clear/increment
schedule function test:schedule_clear/assert 2t
