mod network;

use iced::widget::{button, column, container, row, text, text_input, vertical_space, pick_list, scrollable, checkbox};
use iced::{Alignment, Application, Command, Element, Length, Settings, Theme, theme};
use tokio::sync::watch;
use uuid::Uuid;
use network::SystemReport;
use serde::{Serialize, Deserialize};
use std::fs;
use std::path::PathBuf;
use std::borrow::Cow;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Language {
    English,
    Chinese,
}

impl Language {
    fn get(&self, key: &str) -> &'static str {
        match (self, key) {
            (Language::Chinese, "nav_share") => "网络共享",
            (Language::Chinese, "nav_forward") => "端口转发",
            (Language::Chinese, "nav_monitor") => "系统监控",
            (Language::Chinese, "nav_about") => "关于",
            (Language::Chinese, "title_share") => "Conduit - 网络共享",
            (Language::Chinese, "title_forward") => "Conduit - 端口转发",
            (Language::Chinese, "title_monitor") => "系统网络概览",
            (Language::Chinese, "label_wan") => "外网接口 (WANs):",
            (Language::Chinese, "label_lan") => "目标接口 (LAN):",
            (Language::Chinese, "label_lan_ip") => "局域网 IP:",
            (Language::Chinese, "btn_start_share") => "开始共享",
            (Language::Chinese, "btn_stop_share") => "停止共享",
            (Language::Chinese, "btn_detect") => "检测状态",
            (Language::Chinese, "btn_refresh_iface") => "刷新接口",
            (Language::Chinese, "btn_refresh") => "刷新",
            (Language::Chinese, "btn_add_new") => "添加新转发",
            (Language::Chinese, "btn_import") => "导入",
            (Language::Chinese, "btn_export") => "导出",
            (Language::Chinese, "status_ready") => "就绪",
            (Language::Chinese, "status_active") => "活跃 (已检测)",
            (Language::Chinese, "label_ip_forward") => "IP 转发 (内核):",
            (Language::Chinese, "label_enabled") => "已开启",
            (Language::Chinese, "label_disabled") => "已关闭",
            (Language::Chinese, "monitor_active_flows") => "Conduit 活跃转发流",
            (Language::Chinese, "monitor_nat_rules") => "NAT 规则 (Masquerade)",
            (Language::Chinese, "monitor_port_rules") => "端口转发规则 (DNAT/Redirect)",
            (Language::Chinese, "monitor_listen_ports") => "活跃监听端口 (TCP/UDP)",
            (Language::Chinese, "msg_det_failed") => "检测失败 (权限不足)",
            (Language::Chinese, "msg_select_wan") => "请选择至少一个外网接口",
            (Language::Chinese, "msg_select_lan") => "请选择目标接口",
            (Language::Chinese, "msg_stopping") => "正在停止...",
            (Language::Chinese, "msg_starting") => "正在启动...",
            (Language::Chinese, "msg_stopped") => "已停止",
            (Language::Chinese, "msg_active_bang") => "活跃!",
            (Language::Chinese, "about_desc") => "一个简单易用的网络工具，让开发板联网和端口转发变得更轻松。",
            
            (Language::English, "nav_share") => "Network Share",
            (Language::English, "nav_forward") => "Port Forwarders",
            (Language::English, "nav_monitor") => "System Monitor",
            (Language::English, "nav_about") => "About",
            (Language::English, "title_share") => "Conduit - Network Share",
            (Language::English, "title_forward") => "Conduit - Port Forwarders",
            (Language::English, "title_monitor") => "System Network Overview",
            (Language::English, "label_wan") => "Sources (WANs):",
            (Language::English, "label_lan") => "Target (LAN):",
            (Language::English, "label_lan_ip") => "LAN IP:",
            (Language::English, "btn_start_share") => "Start Share",
            (Language::English, "btn_stop_share") => "Stop Share",
            (Language::English, "btn_detect") => "Detect Status",
            (Language::English, "btn_refresh_iface") => "Refresh Interfaces",
            (Language::English, "btn_refresh") => "Refresh",
            (Language::English, "btn_add_new") => "Add New",
            (Language::English, "btn_import") => "Import",
            (Language::English, "btn_export") => "Export",
            (Language::English, "status_ready") => "Ready",
            (Language::English, "status_active") => "Active (Detected)",
            (Language::English, "label_ip_forward") => "IP Forwarding (Kernel):",
            (Language::English, "label_enabled") => "ENABLED",
            (Language::English, "label_disabled") => "DISABLED",
            (Language::English, "monitor_active_flows") => "Conduit Active Forwarding Flows",
            (Language::English, "monitor_nat_rules") => "NAT Rules (Masquerade)",
            (Language::English, "monitor_port_rules") => "Port Forward Rules (DNAT/Redirect)",
            (Language::English, "monitor_listen_ports") => "Active Listening Ports (TCP/UDP)",
            (Language::English, "msg_det_failed") => "Detection failed (Permission denied)",
            (Language::English, "msg_select_wan") => "Select at least one WAN",
            (Language::English, "msg_select_lan") => "Select LAN interface",
            (Language::English, "msg_stopping") => "Stopping...",
            (Language::English, "msg_starting") => "Starting...",
            (Language::English, "msg_stopped") => "Stopped",
            (Language::English, "msg_active_bang") => "Active!",
            (Language::English, "about_desc") => "A simple and easy-to-use network utility that makes dev-board networking and port forwarding a breeze.",
            _ => "Unknown",
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct SidebarStyle;

impl container::StyleSheet for SidebarStyle {
    type Style = Theme;
    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(iced::Background::Color(iced::Color::from_rgb(0.96, 0.96, 0.98))),
            border: iced::Border {
                width: 0.0,
                ..Default::default()
            },
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct BadgeStyle;

impl container::StyleSheet for BadgeStyle {
    type Style = Theme;
    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(iced::Background::Color(iced::Color::from_rgb(0.2, 0.5, 0.8))),
            border: iced::Border {
                radius: 10.0.into(),
                ..Default::default()
            },
            text_color: Some(iced::Color::WHITE),
            ..Default::default()
        }
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct ContentStyle;

impl container::StyleSheet for ContentStyle {
    type Style = Theme;
    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        container::Appearance {
            background: Some(iced::Background::Color(iced::Color::WHITE)),
            ..Default::default()
        }
    }
}

pub fn main() -> iced::Result {
    tracing_subscriber::fmt::init();
    ForwarderApp::run(Settings {
        fonts: vec![include_bytes!("../assets/fonts/LXGWWenKaiLite-Regular.ttf").as_slice().into()],
        default_font: iced::Font::with_name("LXGW WenKai Lite"),
        ..Settings::default()
    })
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
    language: Language,
    
    // 系统转发
    interfaces: Vec<String>,
    selected_wans: Vec<String>,
    lan_interface: Option<String>,
    host_ip: String,
    subnet_mask: String,
    sys_active: bool,
    sys_status: Cow<'static, str>,

    // 系统监控报告
    system_report: Option<SystemReport>,
    refresh_interval: u64,

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
    SetRefreshInterval(u64),
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
    LanguageChanged(Language),
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

        let (sys_active, active_wans, _) = network::detect_system_forward_status();
        let report = network::get_system_network_report();
        let default_lang = Language::Chinese;

        (
            Self {
                current_page: Page::SystemForward,
                language: default_lang,
                interfaces: ifaces,
                selected_wans: active_wans,
                lan_interface: None,
                host_ip: "192.168.10.1".to_string(),
                subnet_mask: "24".to_string(),
                sys_active,
                sys_status: if sys_active { default_lang.get("status_active").into() } else { default_lang.get("status_ready").into() },
                system_report: Some(report),
                refresh_interval: 1,
                port_forwarders: vec![],
            },
            Command::none(),
        )
    }

    fn title(&self) -> String { "Conduit".to_string() }

    fn update(&mut self, message: Message) -> Command<Message> {
        match message {
            Message::LanguageChanged(lang) => {
                self.language = lang;
                // 刷新系统状态文字
                let status_key = if self.sys_active { "status_active" } else { "status_ready" };
                // 如果当前正在显示某些临时消息（如错误提示），简单刷新为就绪/活跃状态
                self.sys_status = self.language.get(status_key).into();
            }
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
            Message::SetRefreshInterval(interval) => {
                self.refresh_interval = interval;
            }
            Message::DetectSystemForward => {
                let (active, wans, failed) = network::detect_system_forward_status();
                
                // 如果检测失败（通常是权限问题），我们不应该盲目地将状态设为 Inactive
                // 而是保留当前的 sys_active 状态，并给出提示
                if failed {
                    self.sys_status = self.language.get("msg_det_failed").into();
                } else {
                    self.sys_active = active;
                    if active && !wans.is_empty() {
                        self.selected_wans = wans;
                    }
                    let status_key = if active { "status_active" } else { "status_ready" };
                    self.sys_status = self.language.get(status_key).into();
                }
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
                    if wans.is_empty() { self.sys_status = self.language.get("msg_select_wan").into(); return Command::none(); }
                    self.sys_status = if active { self.language.get("msg_stopping").into() } else { self.language.get("msg_starting").into() };
                    return Command::perform(async move {
                        let res = if active { network::stop_system_forwarding(wans, &l) } 
                                 else { network::start_system_forwarding(wans, &l, &host_ip, &mask) };
                        res.map_err(|e| e.to_string())
                    }, move |res| Message::SysForwardingResult(!active, res));
                } else { self.sys_status = self.language.get("msg_select_lan").into(); }
            }
            Message::SysForwardingResult(target, res) => {
                match res {
                    Ok(_) => { self.sys_active = target; self.sys_status = if target { self.language.get("msg_active_bang").into() } else { self.language.get("msg_stopped").into() }; }
                    Err(e) => self.sys_status = format!("{}: {}", if self.language == Language::Chinese { "错误" } else { "Error" }, e).into(),
                }
            }

            Message::AddForwarder => {
                self.port_forwarders.push(PortForwarder {
                    id: Uuid::new_v4(), protocol: Protocol::TCP, src_addr: "0.0.0.0".to_string(), src_port: "".to_string(),
                    dst_addr: "127.0.0.1".to_string(), dst_port: "".to_string(), is_active: false, status: self.language.get("status_ready").to_string(), stop_tx: None,
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
                                    status: format!("{} (Imported)", self.language.get("status_ready")),
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

    fn subscription(&self) -> iced::Subscription<Message> {
        if self.current_page == Page::SystemMonitor {
            iced::time::every(std::time::Duration::from_secs(self.refresh_interval)).map(|_| Message::RefreshSystemReport)
        } else {
            iced::Subscription::none()
        }
    }

    fn view(&self) -> Element<Message> {
        let lang = self.language;

        // 侧边栏按钮样式
        let sidebar_button = |label: &str, icon: &str, page: Page, current_page: Page| {
            let is_selected = page == current_page;
            button(
                row![
                    text(icon).size(16).shaping(iced::widget::text::Shaping::Advanced),
                    text(label).size(14),
                ]
                .spacing(10)
                .align_items(Alignment::Center)
            )
            .width(Length::Fill)
            .padding(12)
            .on_press(Message::SwitchPage(page))
            .style(if is_selected { theme::Button::Primary } else { theme::Button::Text })
        };

        // 侧边栏
        let sidebar = container(
            column![
                text("CONDUIT").size(24).style(theme::Text::Color(iced::Color::from_rgb(0.2, 0.4, 0.7))),
                vertical_space().height(30),
                sidebar_button(lang.get("nav_share"), "🌐", Page::SystemForward, self.current_page),
                sidebar_button(lang.get("nav_forward"), "🔌", Page::PortForward, self.current_page),
                sidebar_button(lang.get("nav_monitor"), "📊", Page::SystemMonitor, self.current_page),
                sidebar_button(lang.get("nav_about"), "ℹ️", Page::About, self.current_page),
                vertical_space().height(Length::Fill),
                row![
                    button("中").on_press(Message::LanguageChanged(Language::Chinese)).style(if self.language == Language::Chinese { theme::Button::Primary } else { theme::Button::Secondary }).padding(5),
                    button("EN").on_press(Message::LanguageChanged(Language::English)).style(if self.language == Language::English { theme::Button::Primary } else { theme::Button::Secondary }).padding(5),
                ].spacing(5).align_items(Alignment::Center)
            ]
            .spacing(10)
            .padding(20)
        )
        .width(200)
        .height(Length::Fill)
        .style(theme::Container::Custom(Box::new(SidebarStyle)));

        let content_area: Element<Message> = match self.current_page {
            Page::About => {
                container(
                    column![
                        container(text("🚀").size(60).shaping(iced::widget::text::Shaping::Advanced)).padding(10),
                        text("Conduit").size(40).style(theme::Text::Color(iced::Color::from_rgb(0.2, 0.4, 0.7))),
                        text(format!("v0.2.1")).size(14).style(theme::Text::Color(iced::Color::from_rgb(0.5, 0.5, 0.5))),
                        vertical_space().height(20),
                        text(lang.get("about_desc")).size(16),
                        vertical_space().height(30),
                        text("GitHub: github.com/xjimlinx/Conduit").size(12),
                        text("Built with Iced & Tokio").size(12).style(theme::Text::Color(iced::Color::from_rgb(0.6, 0.6, 0.6))),
                    ]
                    .spacing(10)
                    .align_items(Alignment::Center)
                )
                .width(Length::Fill)
                .height(Length::Fill)
                .center_x()
                .center_y()
                .into()
            }
            Page::SystemMonitor => {
                if let Some(report) = &self.system_report {
                    let section_card = |title: String, items: &Vec<String>| {
                        let content: Element<Message> = if items.is_empty() {
                            text("No active data").size(12).style(theme::Text::Color(iced::Color::from_rgb(0.5, 0.5, 0.5))).into()
                        } else {
                            let elements: Vec<Element<Message>> = items.iter().map(|i| 
                                container(text(i).size(11).font(iced::Font::MONOSPACE))
                                    .padding([2, 5])
                                    .into()
                            ).collect();
                            column(elements).spacing(4).into()
                        };

                        let card: Element<Message> = container(column![
                            text(title).size(16).style(theme::Text::Color(iced::Color::from_rgb(0.2, 0.4, 0.7))),
                            vertical_space().height(8),
                            content
                        ])
                        .width(Length::Fill)
                        .padding(15)
                        .style(theme::Container::Box)
                        .into();
                        
                        card
                    };

                    column![
                        row![
                            text(lang.get("title_monitor")).size(28),
                            iced::widget::horizontal_space().width(Length::Fill),
                            row![
                                text(format!("{} {}s", if lang.get("nav_share") == "网络共享" { "刷新频率:" } else { "Interval:" }, self.refresh_interval)).size(12),
                                button("1s").on_press(Message::SetRefreshInterval(1)).style(if self.refresh_interval == 1 { theme::Button::Primary } else { theme::Button::Secondary }),
                                button("5s").on_press(Message::SetRefreshInterval(5)).style(if self.refresh_interval == 5 { theme::Button::Primary } else { theme::Button::Secondary }),
                                button("10s").on_press(Message::SetRefreshInterval(10)).style(if self.refresh_interval == 10 { theme::Button::Primary } else { theme::Button::Secondary }),
                            ].spacing(5).align_items(Alignment::Center),
                            button(lang.get("btn_refresh")).on_press(Message::RefreshSystemReport),
                        ].spacing(15).align_items(Alignment::Center),
                        
                        container(row![
                            text(lang.get("label_ip_forward")).size(16),
                            iced::widget::horizontal_space().width(10),
                            text(if report.ip_forward_enabled { lang.get("label_enabled") } else { lang.get("label_disabled") })
                                .size(14)
                                .style(theme::Text::Color(if report.ip_forward_enabled { iced::Color::from_rgb(0.2, 0.6, 0.2) } else { iced::Color::from_rgb(0.7, 0.2, 0.2) }))
                        ].align_items(Alignment::Center))
                        .padding(10)
                        .style(theme::Container::Box),

                        scrollable(column![
                            section_card(lang.get("monitor_active_flows").to_string(), &report.active_connections),
                            section_card(lang.get("monitor_nat_rules").to_string(), &report.nat_masquerade),
                            section_card(lang.get("monitor_port_rules").to_string(), &report.port_forwards),
                            section_card(lang.get("monitor_listen_ports").to_string(), &report.listening_ports),
                        ].spacing(20)).height(Length::Fill),
                    ].spacing(20).into()
                } else {
                    container(text("Loading System Report...").size(20))
                        .width(Length::Fill)
                        .height(Length::Fill)
                        .center_x()
                        .center_y()
                        .into()
                }
            }
            Page::SystemForward => {
                let wan_list = self.interfaces.iter().filter(|iface| Some((*iface).clone()) != self.lan_interface).fold(column![].spacing(5), |col, iface| {
                    col.push(checkbox(iface, self.selected_wans.contains(iface)).on_toggle(move |a| Message::WanToggled(iface.clone(), a)))
                });

                container(column![
                    text(lang.get("title_share")).size(28),
                    vertical_space().height(10),
                    container(column![
                        text(lang.get("label_wan")).size(16).style(theme::Text::Color(iced::Color::from_rgb(0.2, 0.4, 0.7))),
                        scrollable(wan_list).height(120),
                    ].spacing(10)).padding(15).style(theme::Container::Box),
                    
                    container(column![
                        row![text(lang.get("label_lan")).width(120), pick_list(&self.interfaces[..], self.lan_interface.clone(), Message::LanSelected).width(Length::Fill)].spacing(10).align_items(Alignment::Center),
                        row![text(lang.get("label_lan_ip")).width(120), text_input("192.168.10.1", &self.host_ip).on_input(Message::HostIpChanged), text("/"), text_input("24", &self.subnet_mask).on_input(Message::SubnetMaskChanged).width(50)].spacing(10).align_items(Alignment::Center),
                    ].spacing(15)).padding(15).style(theme::Container::Box),

                    row![
                        button(if self.sys_active { lang.get("btn_stop_share") } else { lang.get("btn_start_share") }).on_press(Message::ToggleSysForwarding).width(Length::Fill).padding(12).style(if self.sys_active { theme::Button::Destructive } else { theme::Button::Primary }),
                        button(lang.get("btn_detect")).on_press(Message::DetectSystemForward).padding(12),
                    ].spacing(10),
                    
                    container(row![
                        text("🔔").size(14).shaping(iced::widget::text::Shaping::Advanced),
                        text(&self.sys_status).size(13),
                    ].spacing(10).align_items(Alignment::Center)).padding(10).style(theme::Container::Box),
                    
                    button(lang.get("btn_refresh_iface")).on_press(Message::RefreshInterfaces).style(theme::Button::Secondary),
                ].spacing(20).max_width(600))
                .width(Length::Fill)
                .height(Length::Fill)
                .into()
            }
            Page::PortForward => {
                let list = self.port_forwarders.iter().fold(column![].spacing(15), |col, f| {
                    col.push(container(column![
                        row![
                            container(text(f.protocol.to_string()).size(12))
                                .padding([2, 8])
                                .style(theme::Container::Custom(Box::new(BadgeStyle))),
                            text_input("Src IP", &f.src_addr).on_input(move |v| Message::SrcAddrChanged(f.id, v)).width(Length::Fill),
                            text(":"),
                            text_input("Port", &f.src_port).on_input(move |v| Message::SrcPortChanged(f.id, v)).width(70),
                            text(" ➔ ").size(18),
                            text_input("Dst IP", &f.dst_addr).on_input(move |v| Message::DstAddrChanged(f.id, v)).width(Length::Fill),
                            text(":"),
                            text_input("Port", &f.dst_port).on_input(move |v| Message::DstPortChanged(f.id, v)).width(70)
                        ].spacing(10).align_items(Alignment::Center),
                        row![
                            text(format!("● {}", &f.status)).size(12).style(theme::Text::Color(if f.is_active { iced::Color::from_rgb(0.2, 0.7, 0.2) } else { iced::Color::from_rgb(0.6, 0.6, 0.6) })).width(Length::Fill),
                            button(if f.is_active { text("⏹ Stop").shaping(iced::widget::text::Shaping::Advanced) } else { text("▶ Start").shaping(iced::widget::text::Shaping::Advanced) }).on_press(Message::TogglePortForwarding(f.id)).style(if f.is_active { theme::Button::Destructive } else { theme::Button::Primary }).padding([5, 15]),
                            button(text("🗑").shaping(iced::widget::text::Shaping::Advanced)).on_press(Message::RemoveForwarder(f.id)).style(theme::Button::Secondary).padding([5, 10])
                        ].spacing(10).align_items(Alignment::Center)
                    ].spacing(10).padding(15)).style(theme::Container::Box))
                });

                column![
                    row![
                        text(lang.get("title_forward")).size(28), 
                        iced::widget::horizontal_space().width(Length::Fill), 
                        button(text(format!("➕ {}", lang.get("btn_add_new"))).shaping(iced::widget::text::Shaping::Advanced)).on_press(Message::AddForwarder).style(theme::Button::Primary).padding(10),
                        button(lang.get("btn_import")).on_press(Message::ImportConfig).padding(10),
                        button(lang.get("btn_export")).on_press(Message::ExportConfig).padding(10),
                    ].spacing(10).align_items(Alignment::Center), 
                    scrollable(list).height(Length::Fill)
                ].spacing(20).into()
            }
        };

        row![
            sidebar,
            container(content_area)
                .padding(30)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(theme::Container::Custom(Box::new(ContentStyle)))
        ].into()
    }
}
