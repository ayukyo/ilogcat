mod ui;
mod log;
mod filter;
mod ssh;
mod config;
mod app;
mod app_tabs;
mod i18n;
mod export;
mod stats;

use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;
use clap::Parser;

use crate::app_tabs::{AppState, build_ui};
use crate::i18n::{Language, init as init_i18n};
use crate::ssh::config::SshConfig;

const APP_ID: &str = "com.openclaw.ilogcat";

/// iLogCat - Linux Log Viewer
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// SSH connection string (format: user@host[:port])
    #[arg(short, long, value_name = "SSH_URI")]
    ssh: Option<String>,

    /// SSH password (if not provided, will prompt in GUI)
    #[arg(short = 'P', long)]
    password: Option<String>,
}

/// 解析 SSH URI (格式: user@host[:port])
fn parse_ssh_uri(uri: &str) -> Option<(String, String, u16)> {
    let parts: Vec<&str> = uri.split('@').collect();
    if parts.len() != 2 {
        return None;
    }
    let username = parts[0].to_string();

    let host_parts: Vec<&str> = parts[1].split(':').collect();
    let host = host_parts[0].to_string();
    let port = if host_parts.len() > 1 {
        host_parts[1].parse().unwrap_or(22)
    } else {
        22
    };

    Some((username, host, port))
}

fn main() {
    let args = Args::parse();

    // 加载配置
    let config = config::Config::load().unwrap_or_default();

    // 初始化国际化
    let lang = Language::from_str(&config.ui.language);
    init_i18n(lang);

    // 创建 GTK 应用
    let app = gtk4::Application::builder()
        .application_id(APP_ID)
        .build();

    // 创建应用状态
    let state = Rc::new(RefCell::new(AppState::new()));

    // 处理命令行 SSH 连接
    let ssh_config_from_cli = if let Some(ssh_uri) = &args.ssh {
        if let Some((username, host, port)) = parse_ssh_uri(ssh_uri) {
            let password = args.password.clone().unwrap_or_default();
            Some(SshConfig::new(
                format!("{}@{}", username, host),
                host,
                username,
            )
            .with_port(port)
            .with_password(password))
        } else {
            eprintln!("Invalid SSH URI format. Expected: user@host[:port]");
            None
        }
    } else {
        None
    };

    let ssh_config_clone = ssh_config_from_cli.clone();

    app.connect_activate(move |app| {
        build_ui(app, state.clone());

        // 如果有命令行指定的 SSH 连接，自动启动
        if let Some(ref ssh_cfg) = ssh_config_clone {
            if let Some(ref tm) = state.borrow().tab_manager {
                if let Some(tab) = tm.borrow().current_tab() {
                    use crate::log::LogSource;
                    use crate::log::remote::SshSource;
                    use crate::ui::TabSourceType as SourceType;

                    tab.borrow_mut().set_source_info(SourceType::Ssh(
                        ssh_cfg.host.clone(),
                        "journalctl".to_string()
                    ));

                    let mut source = SshSource::new(
                        ssh_cfg.clone(),
                        "journalctl -f -o short-iso".to_string()
                    );

                    if let Err(e) = source.start() {
                        eprintln!("Failed to start SSH connection: {}", e);
                    } else {
                        tab.borrow_mut().set_source(std::boxed::Box::new(source));
                        if let Some(ref tm) = state.borrow().tab_manager {
                            tm.borrow().update_tab_title(tab.borrow().id);
                        }
                    }
                }
            }
        }
    });

    app.run();
}
