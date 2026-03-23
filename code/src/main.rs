mod ui;
mod log;
mod filter;
mod ssh;
mod config;
mod app;
mod app_tabs;
mod i18n;
mod export;
mod bookmark;

use gtk4::prelude::*;
use std::cell::RefCell;
use std::rc::Rc;

use crate::app_tabs::{AppState, build_ui};
use crate::i18n::{Language, init as init_i18n};

const APP_ID: &str = "com.openclaw.ilogcat";

fn main() {
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

    app.connect_activate(move |app| {
        build_ui(app, state.clone());
    });

    app.run();
}
