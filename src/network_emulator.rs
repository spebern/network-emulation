use std::process::Command;

pub fn setup_network_emulator(rate_kbit: u32, delay_ms: u32) {
    tear_down();

    // create the basic qdisk
    Command::new("/bin/sudo")
        .args(&[
            "/bin/tc", "qdisc", "add", "dev", "lo", "root", "handle", "1:", "htb",
        ])
        .output()
        .expect("faile to crated base qdisk");
    Command::new("/bin/sudo")
        .args(&[
            "/bin/tc", "class", "add", "dev", "lo", "parent", "1:", "classid", "1:1", "htb",
            "rate", "1000Mbps",
        ])
        .output()
        .expect("failed to create base class");

    // master channel
    setup_channel(2, rate_kbit, delay_ms, 13370, 10);

    // slave channel
    setup_channel(3, rate_kbit, delay_ms, 13380, 10);
}

fn tear_down() {
    let _ = Command::new("/bin/sudo")
        .args(&["/bin/tc", "qdisc", "del", "dev", "lo", "root"])
        .output();
}

fn setup_channel(id: usize, rate_kbs: u32, delay_ms: u32, src_port_from: u16, num_ports: u16) {
    Command::new("/bin/sudo")
        .args(&[
            "/bin/tc",
            "class",
            "add",
            "dev",
            "lo",
            "parent",
            "1:1",
            "classid",
            &format!("1:{}", id),
            "htb",
            "rate",
            "1000Mbps",
        ])
        .output()
        .expect("failed to create class");

    Command::new("/bin/sudo")
        .args(&[
            "/bin/tc",
            "qdisc",
            "add",
            "dev",
            "lo",
            "handle",
            &format!("{}:", id),
            "parent",
            &format!("1:{}", id),
            "netem",
            "delay",
            &format!("{}ms", delay_ms),
            "rate",
            &format!("{}kbit", rate_kbs),
        ])
        .output()
        .expect("failed to create qdisk");

    for src_port in src_port_from..src_port_from + num_ports {
        Command::new("/bin/sudo")
            .args(&[
                "/bin/tc",
                "filter",
                "add",
                "dev",
                "lo",
                "pref",
                &format!("{}", id),
                "protocol",
                "ip",
                "u32",
                "match",
                "ip",
                "sport",
                &format!("{}", src_port),
                "0xffff",
                "flowid",
                &format!("1:{}", id),
            ])
            .output()
            .expect("failed to create filter");
    }
}
