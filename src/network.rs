use tokio::net::{TcpListener, TcpStream, UdpSocket};
use tokio::io::copy_bidirectional;
use anyhow::Result;
use tracing::{info, error};
use tokio::sync::watch;
use pnet::datalink;
use std::process::Command;
use std::sync::Arc;
use std::collections::HashMap;
use std::net::SocketAddr;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};

// --- 系统转发相关 ---

pub struct InterfaceInfo {
    pub name: String,
}

pub fn get_interfaces() -> Vec<InterfaceInfo> {
    datalink::interfaces()
        .into_iter()
        .map(|iface| InterfaceInfo {
            name: iface.name,
        })
        .collect()
}

pub fn start_system_forwarding(
    wan_ifs: Vec<String>,
    lan_if: &str,
    host_ip: &str,
    mask: &str,
) -> std::io::Result<()> {
    let mut commands = Vec::new();
    commands.push("echo 1 > /proc/sys/net/ipv4/ip_forward".to_string());
    commands.push(format!("ip addr flush dev {}", lan_if));
    commands.push(format!("ip addr add {}/{} dev {}", host_ip, mask, lan_if));
    commands.push(format!("ip link set {} up", lan_if));

    for wan_if in wan_ifs {
        commands.push(format!("iptables -t nat -D POSTROUTING -o {} -j MASQUERADE || true", wan_if));
        commands.push(format!("iptables -t nat -A POSTROUTING -o {} -j MASQUERADE", wan_if));
        
        commands.push(format!("iptables -D FORWARD -i {} -o {} -m state --state RELATED,ESTABLISHED -j ACCEPT || true", wan_if, lan_if));
        commands.push(format!("iptables -A FORWARD -i {} -o {} -m state --state RELATED,ESTABLISHED -j ACCEPT", wan_if, lan_if));
        
        commands.push(format!("iptables -D FORWARD -i {} -o {} -j ACCEPT || true", lan_if, wan_if));
        commands.push(format!("iptables -A FORWARD -i {} -o {} -j ACCEPT", lan_if, wan_if));
    }
    run_batch_as_root(commands)
}

pub fn stop_system_forwarding(wan_ifs: Vec<String>, lan_if: &str) -> std::io::Result<()> {
    let mut commands = Vec::new();
    for wan_if in wan_ifs {
        commands.push(format!("iptables -t nat -D POSTROUTING -o {} -j MASQUERADE || true", wan_if));
        commands.push(format!("iptables -D FORWARD -i {} -o {} -m state --state RELATED,ESTABLISHED -j ACCEPT || true", wan_if, lan_if));
        commands.push(format!("iptables -D FORWARD -i {} -o {} -j ACCEPT || true", lan_if, wan_if));
    }
    run_batch_as_root(commands)
}

fn run_batch_as_root(commands: Vec<String>) -> std::io::Result<()> {
    if commands.is_empty() { return Ok(()); }
    let full_script = commands.join(" && ");
    let status = Command::new("pkexec").arg("sh").arg("-c").arg(full_script).status()?;
    if status.success() { Ok(()) } else { Err(std::io::Error::new(std::io::ErrorKind::Other, "Root failed")) }
}

// --- TCP/UDP 转发逻辑保持不变 ---
pub async fn start_tcp_forward(src_addr: String, src_port: u16, dst_addr: String, dst_port: u16, mut stop_rx: watch::Receiver<bool>) -> Result<()> {
    let src_socket = format!("{}:{}", src_addr, src_port);
    let dst_socket = format!("{}:{}", dst_addr, dst_port);
    let listener = TcpListener::bind(&src_socket).await?;
    loop {
        tokio::select! {
            accept_res = listener.accept() => {
                if let Ok((mut client, _)) = accept_res {
                    let d = dst_socket.clone();
                    tokio::spawn(async move {
                        if let Ok(mut server) = TcpStream::connect(&d).await {
                            let _ = copy_bidirectional(&mut client, &mut server).await;
                        }
                    });
                }
            }
            _ = stop_rx.changed() => { if *stop_rx.borrow() { break; } }
        }
    }
    Ok(())
}

pub async fn start_udp_forward(src_addr: String, src_port: u16, dst_addr: String, dst_port: u16, mut stop_rx: watch::Receiver<bool>) -> Result<()> {
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
                    let target = if let Some(c) = guard.get_mut(&addr) { c.1 = Instant::now(); c.0.clone() } else {
                        let t = Arc::new(UdpSocket::bind("0.0.0.0:0").await?);
                        t.connect(&dst_socket_addr).await?;
                        let s_clone = socket.clone();
                        let t_clone = t.clone();
                        tokio::spawn(async move {
                            let mut b = [0u8; 4096];
                            while let Ok(n) = t_clone.recv(&mut b).await { let _ = s_clone.send_to(&b[..n], addr).await; }
                        });
                        guard.insert(addr, (t.clone(), Instant::now())); t
                    };
                    let _ = target.send(&buf[..len]).await;
                }
            }
            _ = stop_rx.changed() => { if *stop_rx.borrow() { break; } }
            _ = tokio::time::sleep(Duration::from_secs(30)) => {
                let mut guard = clients.lock().await;
                guard.retain(|_, (_, t)| t.elapsed() < Duration::from_secs(60));
            }
        }
    }
    Ok(())
}
