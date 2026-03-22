mod ui;
mod log;
mod filter;
mod ssh;
mod config;

use gtk4::prelude::*;
use gtk4::Application;

const APP_ID: &str = "com.openclaw.ilogcat";

fn main() {
    // Create a new application
    let app = Application::builder()
        .application_id(APP_ID)
        .build();

    // Connect to "activate" signal
    app.connect_activate(build_ui);

    // Run the application
    app.run();
}

fn build_ui(app: &Application) {
    // Create a window
    let window = gtk4::ApplicationWindow::builder()
        .application(app)
        .title("iLogCat")
        .default_width(1000)
        .default_height(600)
        .build();

    // Create the main vertical box
    let vbox = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Vertical)
        .spacing(6)
        .margin_top(6)
        .margin_bottom(6)
        .margin_start(6)
        .margin_end(6)
        .build();

    // Create toolbar
    let toolbar = create_toolbar();
    vbox.append(&toolbar);

    // Create filter bar
    let filter_bar = create_filter_bar();
    vbox.append(&filter_bar);

    // Create log view (scrolled text view)
    let scrolled = gtk4::ScrolledWindow::builder()
        .hexpand(true)
        .vexpand(true)
        .build();

    let text_view = gtk4::TextView::builder()
        .editable(false)
        .monospace(true)
        .wrap_mode(gtk4::WrapMode::WordChar)
        .build();

    // Create text buffer and tags for log levels
    let buffer = text_view.buffer();
    setup_log_tags(&buffer);

    scrolled.set_child(Some(&text_view));
    vbox.append(&scrolled);

    // Create status bar
    let status_bar = gtk4::Statusbar::new();
    status_bar.push(
        status_bar.context_id("main"),
        "Ready - No log source connected",
    );
    vbox.append(&status_bar);

    // Set the box as window content
    window.set_child(Some(&vbox));

    // Show the window
    window.present();
}

fn create_toolbar() -> gtk4::Box {
    let toolbar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .build();

    // Device/Source selector
    let source_label = gtk4::Label::builder()
        .label("Source:")
        .build();
    toolbar.append(&source_label);

    let source_combo = gtk4::DropDown::from_strings(&[
        "Local: dmesg",
        "Local: journalctl",
        "File...",
        "SSH...",
    ]);
    toolbar.append(&source_combo);

    // Separator
    let sep = gtk4::Separator::new(gtk4::Orientation::Vertical);
    toolbar.append(&sep);

    // Log level selector
    let level_label = gtk4::Label::builder()
        .label("Level:")
        .build();
    toolbar.append(&level_label);

    let level_combo = gtk4::DropDown::from_strings(&[
        "Verbose",
        "Debug",
        "Info",
        "Warn",
        "Error",
        "Fatal",
    ]);
    level_combo.set_selected(2); // Default: Info
    toolbar.append(&level_combo);

    // Buttons
    let clear_btn = gtk4::Button::builder()
        .label("Clear")
        .build();
    toolbar.append(&clear_btn);

    let pause_btn = gtk4::Button::builder()
        .label("Pause")
        .build();
    toolbar.append(&pause_btn);

    let settings_btn = gtk4::Button::builder()
        .icon_name("preferences-system-symbolic")
        .build();
    toolbar.append(&settings_btn);

    toolbar
}

fn create_filter_bar() -> gtk4::Box {
    let filter_bar = gtk4::Box::builder()
        .orientation(gtk4::Orientation::Horizontal)
        .spacing(6)
        .build();

    // Search entry
    let search_entry = gtk4::SearchEntry::builder()
        .placeholder_text("Filter logs...")
        .hexpand(true)
        .build();
    filter_bar.append(&search_entry);

    // Add filter button
    let add_filter_btn = gtk4::Button::builder()
        .label("+ Filter")
        .build();
    filter_bar.append(&add_filter_btn);

    // Regex toggle
    let regex_toggle = gtk4::ToggleButton::builder()
        .label("Regex")
        .build();
    filter_bar.append(&regex_toggle);

    // Case sensitive toggle
    let case_toggle = gtk4::ToggleButton::builder()
        .label("Aa")
        .build();
    filter_bar.append(&case_toggle);

    filter_bar
}

fn setup_log_tags(buffer: &gtk4::TextBuffer) {
    let tag_table = buffer.tag_table();

    // Verbose - Gray
    let tag_verbose = gtk4::TextTag::builder()
        .name("verbose")
        .foreground("#808080")
        .build();
    tag_table.add(&tag_verbose);

    // Debug - Blue
    let tag_debug = gtk4::TextTag::builder()
        .name("debug")
        .foreground("#0066CC")
        .build();
    tag_table.add(&tag_debug);

    // Info - Green
    let tag_info = gtk4::TextTag::builder()
        .name("info")
        .foreground("#008800")
        .build();
    tag_table.add(&tag_info);

    // Warn - Orange
    let tag_warn = gtk4::TextTag::builder()
        .name("warn")
        .foreground("#FF8800")
        .build();
    tag_table.add(&tag_warn);

    // Error - Red
    let tag_error = gtk4::TextTag::builder()
        .name("error")
        .foreground("#CC0000")
        .build();
    tag_table.add(&tag_error);

    // Fatal - Red Bold
    let tag_fatal = gtk4::TextTag::builder()
        .name("fatal")
        .foreground("#CC0000")
        .weight(700) // Bold
        .build();
    tag_table.add(&tag_fatal);
}