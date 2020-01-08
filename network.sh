# delete previous config
tc qdisc del dev lo root

tc qdisc add dev lo root handle 1: htb
tc class add dev lo parent 1: classid 1:1 htb rate 1000Mbps

# master
tc class add dev lo parent 1:1 classid 1:2 htb rate 1000Mbps
tc qdisc add dev lo handle 2: parent 1:2 netem delay 3ms rate 500kbit
tc filter add dev lo pref 2 protocol ip u32 match ip sport 13370 0xffff flowid 1:2
tc filter add dev lo pref 2 protocol ip u32 match ip sport 13371 0xffff flowid 1:2
tc filter add dev lo pref 2 protocol ip u32 match ip sport 13372 0xffff flowid 1:2
tc filter add dev lo pref 2 protocol ip u32 match ip sport 13373 0xffff flowid 1:2
tc filter add dev lo pref 2 protocol ip u32 match ip sport 13374 0xffff flowid 1:2
tc filter add dev lo pref 2 protocol ip u32 match ip sport 13375 0xffff flowid 1:2
tc filter add dev lo pref 2 protocol ip u32 match ip sport 13376 0xffff flowid 1:2
tc filter add dev lo pref 2 protocol ip u32 match ip sport 13377 0xffff flowid 1:2
tc filter add dev lo pref 2 protocol ip u32 match ip sport 13378 0xffff flowid 1:2
tc filter add dev lo pref 2 protocol ip u32 match ip sport 13379 0xffff flowid 1:2

# slave
tc class add dev lo parent 1:1 classid 1:3 htb rate 1000Mbps
tc qdisc add dev lo handle 3: parent 1:3 netem delay 3ms
tc filter add dev lo pref 3 protocol ip u32 match ip sport 13380 0xffff flowid 1:3
tc filter add dev lo pref 3 protocol ip u32 match ip sport 13381 0xffff flowid 1:3
tc filter add dev lo pref 3 protocol ip u32 match ip sport 13382 0xffff flowid 1:3
tc filter add dev lo pref 3 protocol ip u32 match ip sport 13383 0xffff flowid 1:3
tc filter add dev lo pref 3 protocol ip u32 match ip sport 13384 0xffff flowid 1:3
tc filter add dev lo pref 3 protocol ip u32 match ip sport 13385 0xffff flowid 1:3
tc filter add dev lo pref 3 protocol ip u32 match ip sport 13386 0xffff flowid 1:3
tc filter add dev lo pref 3 protocol ip u32 match ip sport 13387 0xffff flowid 1:3
tc filter add dev lo pref 3 protocol ip u32 match ip sport 13388 0xffff flowid 1:3
tc filter add dev lo pref 3 protocol ip u32 match ip sport 13389 0xffff flowid 1:3