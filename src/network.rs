use anyhow::Result;
use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::io::copy_bidirectional;
use tokio::sync::{watch, Mutex};
use tokio::time::{Duration, Instant};
use std::process::Command;
use pnet::datalink;

#[derive(Debug, Clone)]
pub struct SystemReport {
    pub ip_forward_enabled: bool,
    pub nat_masquerade: Vec<String>,
    pub port_forwards: Vec<String>,
    pub listening_ports: Vec<String>,
    pub active_connections: Vec<String>,
    pub iptables_failed: bool,
}

pub struct InterfaceInfo {
    pub name: String,
}

pub fn get_interfaces() -> Vec<InterfaceInfo> {
    datalink::interfaces()
        .into_iter()
        .map(|i| InterfaceInfo { name: i.name })
        .collect()
}

// --- Linux 实现 ---
#[cfg(target_os = "linux")]
pub fn get_system_network_report() -> SystemReport {
    let mut report = SystemReport {
        ip_forward_enabled: false,
        nat_masquerade: vec![],
        port_forwards: vec![],
        listening_ports: vec![],
        active_connections: vec![],
        iptables_failed: false,
    };

    report.ip_forward_enabled = std::fs::read_to_string("/proc/sys/net/ipv4/ip_forward")
        .unwrap_or_default().trim() == "1";

    if let Ok(output) = Command::new("iptables").args(["-t", "nat", "-S"]).output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("MASQUERADE") {
                    report.nat_masquerade.push(line.to_string());
                } else if line.contains("DNAT") || line.contains("REDIRECT") {
                    report.port_forwards.push(line.to_string());
                }
            }
        } else {
            report.iptables_failed = true;
        }
    } else {
        report.iptables_failed = true;
    }

    if let Ok(output) = Command::new("ss").args(["-tlnpu"]).output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines().skip(1) {
                report.listening_ports.push(line.to_string());
            }
        }
    }

    if let Ok(output) = Command::new("ss").args(["-apn"]).output() {
        if output.status.success() {
            let stdout = String::from_utf8_lossy(&output.stdout);
            for line in stdout.lines() {
                if line.contains("conduit") && (line.contains("ESTAB") || line.contains("LISTEN") || line.contains("UNCONN")) {
                    report.active_connections.push(line.to_string());
                }
            }
        }
    }

    report
}

#[cfg(target_os = "linux")]
pub fn detect_system_forward_status() -> (bool, Vec<String>, bool) {
    let report = get_system_network_report();
    let mut active_wans = Vec::new();
    for rule in &report.nat_masquerade {
        let parts: Vec<&str> = rule.split_whitespace().collect();
        if let Some(pos) = parts.iter().position(|&r| r == "-o") {
            if let Some(iface) = parts.get(pos + 1) {
                active_wans.push(iface.to_string());
            }
        }
    }
    let active = report.ip_forward_enabled && !active_wans.is_empty();
    (active, active_wans, report.iptables_failed)
}

#[cfg(target_os = "linux")]
pub fn start_system_forwarding(wan_ifs: Vec<String>, lan_if: &str, host_ip: &str, mask: &str) -> std::io::Result<()> {
    let mut commands = Vec::new();
    commands.push("echo 1 > /proc/sys/net/ipv4/ip_forward".to_string());
    commands.push(format!("ip addr add {}/{} dev {} 2>/dev/null || true", host_ip, mask, lan_if));
    commands.push(format!("ip link set {} up", lan_if));
    for wan_if in wan_ifs {
        commands.push(format!("iptables -t nat -D POSTROUTING -o {} -j MASQUERADE 2>/dev/null || true", wan_if));
        commands.push(format!("iptables -t nat -A POSTROUTING -o {} -j MASQUERADE", wan_if));
        commands.push(format!("iptables -D FORWARD -i {} -o {} -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || true", wan_if, lan_if));
        commands.push(format!("iptables -A FORWARD -i {} -o {} -m state --state RELATED,ESTABLISHED -j ACCEPT", wan_if, lan_if));
        commands.push(format!("iptables -D FORWARD -i {} -o {} -j ACCEPT 2>/dev/null || true", lan_if, wan_if));
        commands.push(format!("iptables -A FORWARD -i {} -o {} -j ACCEPT", lan_if, wan_if));
    }
    run_batch_as_root(commands)
}

#[cfg(target_os = "linux")]
pub fn stop_system_forwarding(wan_ifs: Vec<String>, lan_if: &str) -> std::io::Result<()> {
    let mut commands = Vec::new();
    for wan_if in wan_ifs {
        commands.push(format!("iptables -t nat -D POSTROUTING -o {} -j MASQUERADE 2>/dev/null || true", wan_if));
        commands.push(format!("iptables -D FORWARD -i {} -o {} -m state --state RELATED,ESTABLISHED -j ACCEPT 2>/dev/null || true", wan_if, lan_if));
        commands.push(format!("iptables -D FORWARD -i {} -o {} -j ACCEPT 2>/dev/null || true", lan_if, wan_if));
    }
    commands.push("echo 0 > /proc/sys/net/ipv4/ip_forward".to_string());
    run_batch_as_root(commands)
}

#[cfg(target_os = "linux")]
fn run_batch_as_root(commands: Vec<String>) -> std::io::Result<()> {
    if commands.is_empty() { return Ok(()); }
    let full_script = commands.join(" && ");
    let status = Command::new("pkexec").arg("sh").arg("-c").arg(full_script).status()?;
    if status.success() { Ok(()) } else { Err(std::io::Error::new(std::io::ErrorKind::Other, "Root failed")) }
}

// --- Windows 实现 (初步框架) ---
#[cfg(target_os = "windows")]
pub fn get_system_network_report() -> SystemReport {
    let mut report = SystemReport {
        ip_forward_enabled: false,
        nat_masquerade: vec![],
        port_forwards: vec![],
        listening_ports: vec![],
        active_connections: vec![],
        iptables_failed: false,
    };

    // 检测注册表查看 IP 转发是否开启 (由于 std 限制，这里先默认 false 或通过 powershell)
    if let Ok(output) = Command::new("powershell")
        .args(["-Command", "(Get-ItemProperty -Path 'HKLM:\\SYSTEM\\CurrentControlSet\\Services\\Tcpip\\Parameters').IPEnableRouter"])
        .output() {
        report.ip_forward_enabled = String::from_utf8_lossy(&output.stdout).trim() == "1";
    }

    // Windows 监听端口检测
    if let Ok(output) = Command::new("netstat").arg("-ano").output() {
        let stdout = String::from_utf8_lossy(&output.stdout);
        for line in stdout.lines().skip(4) {
            report.listening_ports.push(line.to_string());
        }
    }

    // Windows 活跃连接检测 (过滤包含 conduit 的进程，由于 netstat 不直接显示进程名，需要复杂匹配，这里先占位)
    report.active_connections.push("Windows support is in progress...".to_string());

    report
}

#[cfg(target_os = "windows")]
pub fn detect_system_forward_status() -> (bool, Vec<String>, bool) {
    let report = get_system_network_report();
    // Windows 默认先不检测活跃 WAN，待完善
    (report.ip_forward_enabled, vec![], false)
}

#[cfg(target_os = "windows")]
pub fn start_system_forwarding(_wan_ifs: Vec<String>, _lan_if: &str, _host_ip: &str, _mask: &str) -> std::io::Result<()> {
    // Windows 下开启转发需要修改注册表并重启服务，或者使用 New-NetNat (Win10+)
    // 暂不支持一键开启，需手动配置或通过后续复杂脚本实现
    Err(std::io::Error::new(std::io::ErrorKind::Other, "System forwarding is not yet fully supported on Windows. Please use Port Forwarders instead."))
}

#[cfg(target_os = "windows")]
pub fn stop_system_forwarding(_wan_ifs: Vec<String>, _lan_if: &str) -> std::io::Result<()> {
    Ok(())
}

// --- 跨平台通用转发逻辑 (无需 cfg) ---

pub async fn start_tcp_forward(
    src_addr: String,
    src_port: u16,
    dst_addr: String,
    dst_port: u16,
    mut stop_rx: watch::Receiver<bool>,
) -> Result<()> {
    let src_socket = format!("{}:{}", src_addr, src_port);
    let dst_socket = format!("{}:{}", dst_addr, dst_port);
    let listener = TcpListener::bind(&src_socket).await?;

    loop {
        tokio::select! {
            accept_res = listener.accept() => {
                if let Ok((mut client, _)) = accept_res {
                    let d = dst_socket.clone();
                    let mut stop_rx_clone = stop_rx.clone();
                    tokio::spawn(async move {
                        if let Ok(mut server) = TcpStream::connect(&d).await {
                            tokio::select! {
                                _ = copy_bidirectional(&mut client, &mut server) => {},
                                _ = stop_rx_clone.changed() => {},
                            }
                        }
                    });
                }
            }
            _ = stop_rx.changed() => {
                if *stop_rx.borrow() { break; }
            }
        }
    }
    Ok(())
}

pub async fn start_udp_forward(
    src_addr: String,
    src_port: u16,
    dst_addr: String,
    dst_port: u16,
    mut stop_rx: watch::Receiver<bool>,
) -> Result<()> {
    let src_socket_addr = format!("{}:{}", src_addr, src_port);
    let dst_socket_addr = format!("{}:{}", dst_addr, dst_port);
    let socket = Arc::new(UdpSocket::bind(&src_socket_addr).await?);

    let clients: Arc<Mutex<HashMap<SocketAddr, (Arc<UdpSocket>, Instant)>>> = Arc::new(Mutex::new(HashMap::new()));
    let mut buf = [0u8; 4096];

    loop {
        tokio::select! {
            res = socket.recv_from(&mut buf) => {
                if let Ok((len, addr)) = res {
                    let mut guard = clients.lock().await;
                    let target = if let Some(c) = guard.get_mut(&addr) {
                        c.1 = Instant::now();
                        c.0.clone()
                    } else {
                        let t = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
                        t.connect(&dst_socket_addr).await?;
                        
                        let s_clone = socket.clone();
                        let t_clone = t.clone();
                        let mut stop_rx_clone = stop_rx.clone();
                        
                        tokio::spawn(async move {
                            let mut b = [0u8; 4096];
                            loop {
                                tokio::select! {
                                    n_res = t_clone.recv(&mut b) => {
                                        if let Ok(n) = n_res {
                                            let _ = s_clone.send_to(&b[..n], addr).await;
                                        } else { break; }
                                    }
                                    _ = stop_rx_clone.changed() => { break; }
                                }
                            }
                        });
                        guard.insert(addr, (t.clone(), Instant::now()));
                        t
                    };
                    let _ = target.send(&buf[..len]).await;
                }
            }
            _ = stop_rx.changed() => {
                if *stop_rx.borrow() { break; }
            }
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                let mut guard = clients.lock().await;
                guard.retain(|_, (_, t)| t.elapsed() < Duration::from_secs(60));
            }
        }
    }
    Ok(())
}
