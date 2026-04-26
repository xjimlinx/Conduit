mod network;

use iced::widget::{button, column, container, row, text, text_input, vertical_space, pick_list, scrollable, checkbox};
use iced::{Alignment, Application, Command, Element, Length, Settings, Theme, theme};
use tokio::sync::watch;
use uuid::Uuid;
use network::SystemReport;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::PathBuf;

pub fn main() -> iced::Result {
    tracing_subscriber::fmt::init();
    ForwarderApp::run(Settings::default())
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Page {
    SystemForward,
    PortForward,
    SystemMonitor,
    About,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
enum Protocol {
    TCP,
    UDP,
}

impl Protocol {
    const ALL: [Protocol; 2] = [Protocol::TCP, Protocol::UDP];
}

impl std::fmt::Display for Protocol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Serialize, Deserialize, Clone)]
struct PortForwarderConfig {
    pub protocol: Protocol,
    pub src_addr: String,
    pub src_port: String,
    pub dst_addr: String,
    pub dst_port: String,
}

struct PortForwarder {
    id: Uuid,
    protocol: Protocol,
    src_addr: String,
    src_port: String,
    dst_addr: String,
    dst_port: String,
    is_active: bool,
    status: String,
    stop_tx: Option<watch::Sender<bool>>,
}

struct ForwarderApp {
    current_page: Page,
    
    // 系统转发
    interfaces: Vec<String>,
    selected_wans: Vec<String>,
    lan_interface: Option<String>,
    host_ip: String,
    subnet_mask: String,
    sys_active: bool,
    sys_status: String,

    // 系统监控报告
    system_report: Option<SystemReport>,

    // 多端口转发列表
    port_forwarders: Vec<PortForwarder>,
}

#[derive(Debug, Clone)]
enum Message {
    SwitchPage(Page),
    // 系统转发
    WanToggled(String, bool),
    LanSelected(String),
    HostIpChanged(String),
    SubnetMaskChanged(String),
    ToggleSysForwarding,
    SysForwardingResult(bool, Result<(), String>),
    DetectSystemForward,
    RefreshInterfaces,
    // 系统监控
    RefreshSystemReport,
    // 端口转发
    AddForwarder,
    RemoveForwarder(Uuid),
    ProtocolChanged(Uuid, Protocol),
    SrcAddrChanged(Uuid, String),
    SrcPortChanged(Uuid, String),
    DstAddrChanged(Uuid, String),
    DstPortChanged(Uuid, String),
    TogglePortForwarding(Uuid),
    PortForwardingResult(Uuid, Result<(), String>),
    ImportConfig,
    ConfigFileSelected(Option<PathBuf>),
    ExportConfig,
    ConfigFileToExportSelected(Option<PathBuf>),
}

impl Application for ForwarderApp {
    type Message = Message;
    type Theme = Theme;
    type Executor = iced::executor::Default;
    type Flags = ();

    fn new(_flags: ()) -> (Self, Command<Message>) {
        let ifaces: Vec<String> = network::get_interfaces()
            .into_iter()
            .filter(|i| {
                let name = i.name.as_str();
                name != "lo" && 
                !name.starts_with("veth") && 
                !name.starts_with("docker") && 
                !name.starts_with("br-")
            })
            .map(|i| i.name)
            .collect();

        let (sys_active, active_wans) = network::detect_system_forward_status();
        let report = network::get_system_network_report();

        (
            Self {
                current_page: Page::SystemForward,
                interfaces: ifaces,
                selected_wans: active_wans,
                lan_interface: None,
                host_ip: "192.168.10.1".to_string(),
                subnet_mask: "24".to_string(),
                sys_active,
                sys_status: if sys_active { "Active (Detected)".to_string() } else { "Ready".to_string() },
                system_report: Some(report),
                port_forwarders: vec![],
            },
            Command::none(),
        )
    }

    fn title(&self) -> String { "Conduit".to_string() }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::SwitchPage(page) => self.current_page = page,
            Message::RefreshInterfaces => {
                self.interfaces = network::get_interfaces().into_iter().filter(|i| {
                    let n = &i.name;
                    n != "lo" && !n.starts_with("veth") && !n.starts_with("docker") && !n.starts_with("br-")
                }).map(|i| i.name).collect();
            }
            Message::RefreshSystemReport => {
                self.system_report = Some(network::get_system_network_report());
            }
            Message::DetectSystemForward => {
                let (active, wans) = network::detect_system_forward_status();
                self.sys_active = active;
                self.selected_wans = wans;
                self.sys_status = if active { "Active (Detected)".to_string() } else { "Inactive".to_string() };
            }
            Message::WanToggled(name, active) => {
                if active { self.selected_wans.push(name); }
                else { self.selected_wans.retain(|n| n != &name); }
            }
            Message::LanSelected(name) => self.lan_interface = Some(name),
            Message::HostIpChanged(ip) => self.host_ip = ip,
            Message::SubnetMaskChanged(mask) => self.subnet_mask = mask,
            Message::ToggleSysForwarding => {
                let active = self.sys_active;
                let wans = self.selected_wans.clone();
                let lan = self.lan_interface.clone();
                let host_ip = self.host_ip.clone();
                let mask = self.subnet_mask.clone();

                if let Some(l) = lan {
                    if wans.is_empty() { self.sys_status = "Select at least one WAN".to_string(); return Command::none(); }
                    self.sys_status = if active { "Stopping..." } else { "Starting..." }.to_string();
                    return Command::perform(async move {
                        let res = if active { network::stop_system_forwarding(wans, &l) } 
                                 else { network::start_system_forwarding(wans, &l, &host_ip, &mask) };
                        res.map_err(|e| e.to_string())
                    }, move |res| Message::SysForwardingResult(!active, res));
                } else { self.sys_status = "Select LAN interface".to_string(); }
            }
            Message::SysForwardingResult(target, res) => {
                match res {
                    Ok(_) => { self.sys_active = target; self.sys_status = if target { "Active!" } else { "Stopped" }.to_string(); }
                    Err(e) => self.sys_status = format!("Error: {}", e),
                }
            }

            Message::AddForwarder => {
                self.port_forwarders.push(PortForwarder {
                    id: Uuid::new_v4(), protocol: Protocol::TCP, src_addr: "0.0.0.0".to_string(), src_port: "".to_string(),
                    dst_addr: "127.0.0.1".to_string(), dst_port: "".to_string(), is_active: false, status: "Ready".to_string(), stop_tx: None,
                });
            }
            Message::RemoveForwarder(id) => {
                if let Some(pos) = self.port_forwarders.iter().position(|f| f.id == id) {
                    if self.port_forwarders[pos].is_active { if let Some(tx) = self.port_forwarders[pos].stop_tx.take() { let _ = tx.send(true); } }
                    self.port_forwarders.remove(pos);
                }
            }
            Message::ProtocolChanged(id, proto) => if let Some(f) = self.port_forwarders.iter_mut().find(|f| f.id == id) { f.protocol = proto; }
            Message::SrcAddrChanged(id, addr) => if let Some(f) = self.port_forwarders.iter_mut().find(|f| f.id == id) { f.src_addr = addr; }
            Message::SrcPortChanged(id, port) => if let Some(f) = self.port_forwarders.iter_mut().find(|f| f.id == id) { f.src_port = port; }
            Message::DstAddrChanged(id, addr) => if let Some(f) = self.port_forwarders.iter_mut().find(|f| f.id == id) { f.dst_addr = addr; }
            Message::DstPortChanged(id, port) => if let Some(f) = self.port_forwarders.iter_mut().find(|f| f.id == id) { f.dst_port = port; }
            Message::TogglePortForwarding(id) => {
                if let Some(f) = self.port_forwarders.iter_mut().find(|f| f.id == id) {
                    if f.is_active { if let Some(tx) = f.stop_tx.take() { let _ = tx.send(true); } f.is_active = false; f.status = "Stopped".to_string(); } 
                    else {
                        if let (Ok(sp), Ok(dp)) = (f.src_port.parse::<u16>(), f.dst_port.parse::<u16>()) {
                            let (tx, rx) = watch::channel(false); f.stop_tx = Some(tx); f.is_active = true; f.status = "Running".to_string();
                            let s = f.src_addr.clone(); let d = f.dst_addr.clone(); let p = f.protocol;
                            return Command::perform(async move {
                                let res = if p == Protocol::TCP { network::start_tcp_forward(s, sp, d, dp, rx).await }
                                         else { network::start_udp_forward(s, sp, d, dp, rx).await };
                                res.map_err(|e| e.to_string())
                            }, move |res| Message::PortForwardingResult(id, res));
                        } else { f.status = "Invalid port".to_string(); }
                    }
                }
            }
            Message::PortForwardingResult(id, res) => if let Some(f) = self.port_forwarders.iter_mut().find(|f| f.id == id) {
                if let Err(e) = res { f.is_active = false; f.status = format!("Error: {}", e); }
            }
            Message::ImportConfig => {
                return Command::perform(async move {
                    rfd::AsyncFileDialog::new()
                        .add_filter("JSON", &["json"])
                        .pick_file()
                        .await
                        .map(|f| f.path().to_path_buf())
                }, Message::ConfigFileSelected);
            }
            Message::ConfigFileSelected(path) => {
                if let Some(p) = path {
                    if let Ok(content) = fs::read_to_string(p) {
                        if let Ok(configs) = serde_json::from_str::<Vec<PortForwarderConfig>>(&content) {
                            for cfg in configs {
                                self.port_forwarders.push(PortForwarder {
                                    id: Uuid::new_v4(),
                                    protocol: cfg.protocol,
                                    src_addr: cfg.src_addr,
                                    src_port: cfg.src_port,
                                    dst_addr: cfg.dst_addr,
                                    dst_port: cfg.dst_port,
                                    is_active: false,
                                    status: "Ready (Imported)".to_string(),
                                    stop_tx: None,
                                });
                            }
                        }
                    }
                }
            }
            Message::ExportConfig => {
                return Command::perform(async move {
                    rfd::AsyncFileDialog::new()
                        .add_filter("JSON", &["json"])
                        .set_file_name("config.json")
                        .save_file()
                        .await
                        .map(|f| f.path().to_path_buf())
                }, Message::ConfigFileToExportSelected);
            }
            Message::ConfigFileToExportSelected(path) => {
                if let Some(p) = path {
                    let configs: Vec<PortForwarderConfig> = self.port_forwarders.iter().map(|f| PortForwarderConfig {
                        protocol: f.protocol,
                        src_addr: f.src_addr.clone(),
                        src_port: f.src_port.clone(),
                        dst_addr: f.dst_addr.clone(),
                        dst_port: f.dst_port.clone(),
                    }).collect();
                    if let Ok(json) = serde_json::to_string_pretty(&configs) {
                        let _ = fs::write(p, json);
                    }
                }
            }
        }
        Command::none()
    }

    fn view(&self) -> Element<Message> {
        let nav = row![
            button("Network Share").on_press(Message::SwitchPage(Page::SystemForward)).style(if self.current_page == Page::SystemForward { theme::Button::Primary } else { theme::Button::Secondary }),
            button("Port Forwarders").on_press(Message::SwitchPage(Page::PortForward)).style(if self.current_page == Page::PortForward { theme::Button::Primary } else { theme::Button::Secondary }),
            button("System Monitor").on_press(Message::SwitchPage(Page::SystemMonitor)).style(if self.current_page == Page::SystemMonitor { theme::Button::Primary } else { theme::Button::Secondary }),
            button("About").on_press(Message::SwitchPage(Page::About)).style(if self.current_page == Page::About { theme::Button::Primary } else { theme::Button::Secondary }),
        ].spacing(10);

        let content: Element<Message> = match self.current_page {
            Page::About => {
                column![
                    text("Conduit").size(40),
                    text("Version 0.1.0").size(18),
                    vertical_space().height(20),
                    text("A high-performance network utility built with Rust.").size(16),
                    text("GitHub: github.com/xjimlinx/Conduit").size(12),
                    text("Built with Iced & Tokio").size(12),
                ].spacing(10).align_items(Alignment::Center).into()
            }
            Page::SystemMonitor => {
                if let Some(report) = &self.system_report {
                    let section = |title: &str, items: &Vec<String>| {
                        let content: Element<Message> = if items.is_empty() {
                            text("None").size(12).style(theme::Text::Color(iced::Color::from_rgb(0.5, 0.5, 0.5))).into()
                        } else {
                            let elements: Vec<Element<Message>> = items.iter().map(|i| text(i).size(11).into()).collect();
                            column(elements).spacing(2).into()
                        };

                        column![
                            text(title).size(18).style(theme::Text::Color(iced::Color::from_rgb(0.2, 0.5, 0.8))),
                            content
                        ].spacing(5)
                    };

                    column![
                        row![
                            text("System Network Overview").size(25),
                            iced::widget::horizontal_space().width(Length::Fill),
                            button("Refresh").on_press(Message::RefreshSystemReport),
                        ].align_items(Alignment::Center),
                        
                        row![
                            text("IP Forwarding (Kernel): "),
                            text(if report.ip_forward_enabled { "ENABLED" } else { "DISABLED" })
                                .style(theme::Text::Color(if report.ip_forward_enabled { iced::Color::from_rgb(0.0, 0.7, 0.0) } else { iced::Color::from_rgb(0.7, 0.0, 0.0) }))
                        ].spacing(10),

                        scrollable(column![
                            section("Conduit Active Forwarding Flows", &report.active_connections),
                            vertical_space().height(10),
                            section("NAT Rules (Masquerade)", &report.nat_masquerade),
                            vertical_space().height(10),
                            section("Port Forward Rules (DNAT/Redirect)", &report.port_forwards),
                            vertical_space().height(10),
                            section("Active Listening Ports (TCP/UDP)", &report.listening_ports),
                        ].spacing(15)).height(Length::Fill),
                    ].spacing(15).into()
                } else {
                    text("Loading report...").into()
                }
            }
            Page::SystemForward => {
                let wan_list = self.interfaces.iter().filter(|iface| Some((*iface).clone()) != self.lan_interface).fold(column![].spacing(5), |col, iface| {
                    col.push(checkbox(iface, self.selected_wans.contains(iface)).on_toggle(move |a| Message::WanToggled(iface.clone(), a)))
                });

                column![
                    text("Conduit - Network Share").size(25),
                    text("Sources (WANs):").size(16),
                    scrollable(wan_list).height(100),
                    row![text("Target (LAN): ").width(100), pick_list(&self.interfaces[..], self.lan_interface.clone(), Message::LanSelected).width(Length::Fill)].spacing(10).align_items(Alignment::Center),
                    row![text("LAN IP: ").width(100), text_input("192.168.10.1", &self.host_ip).on_input(Message::HostIpChanged), text("/"), text_input("24", &self.subnet_mask).on_input(Message::SubnetMaskChanged).width(40)].spacing(10).align_items(Alignment::Center),
                    row![
                        button(if self.sys_active { "Stop Share" } else { "Start Share" }).on_press(Message::ToggleSysForwarding).width(Length::Fill).style(if self.sys_active { theme::Button::Destructive } else { theme::Button::Primary }),
                        button("Detect Status").on_press(Message::DetectSystemForward),
                    ].spacing(10),
                    text(&self.sys_status),
                    button("Refresh Interfaces").on_press(Message::RefreshInterfaces),
                ].spacing(15).max_width(500).into()
            }
            Page::PortForward => {
                let list = self.port_forwarders.iter().fold(column![].spacing(10), |col, f| {
                    col.push(container(column![
                        row![pick_list(&Protocol::ALL[..], Some(f.protocol), move |p| Message::ProtocolChanged(f.id, p)).width(80), text_input("Src IP", &f.src_addr).on_input(move |v| Message::SrcAddrChanged(f.id, v)).width(Length::Fill), text(":"), text_input("Port", &f.src_port).on_input(move |v| Message::SrcPortChanged(f.id, v)).width(60), text("->"), text_input("Dst IP", &f.dst_addr).on_input(move |v| Message::DstAddrChanged(f.id, v)).width(Length::Fill), text(":"), text_input("Port", &f.dst_port).on_input(move |v| Message::DstPortChanged(f.id, v)).width(60)].spacing(5).align_items(Alignment::Center),
                        row![text(&f.status).size(12).width(Length::Fill), button(if f.is_active { "Stop" } else { "Start" }).on_press(Message::TogglePortForwarding(f.id)).style(if f.is_active { theme::Button::Destructive } else { theme::Button::Primary }), button("Delete").on_press(Message::RemoveForwarder(f.id)).style(theme::Button::Secondary)].spacing(10).align_items(Alignment::Center)
                    ].padding(10)).style(theme::Container::Box))
                });
                column![
                    row![
                        text("Conduit - Port Forwarders").size(25), 
                        iced::widget::horizontal_space().width(Length::Fill), 
                        button("Add New").on_press(Message::AddForwarder),
                        button("Import").on_press(Message::ImportConfig),
                        button("Export").on_press(Message::ExportConfig),
                    ].spacing(10).align_items(Alignment::Center), 
                    scrollable(list).height(Length::Fill)
                ].spacing(15).into()
            }
        };

        container(column![nav, vertical_space().height(20), content].padding(20)).width(Length::Fill).height(Length::Fill).center_x().into()
    }
}
