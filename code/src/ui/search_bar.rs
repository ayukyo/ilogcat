use gtk4::prelude::*;
use gtk4::{Box, Orientation, Entry, Button, Label, TextBuffer, TextIter, TextTag};
use std::cell::RefCell;
use std::rc::Rc;

/// 搜索栏组件
pub struct SearchBar {
    pub container: Box,
    pub search_entry: Entry,
    pub prev_btn: Button,
    pub next_btn: Button,
    pub close_btn: Button,
    pub match_label: Label,
    search_text: Rc<RefCell<String>>,
    current_match: Rc<RefCell<i32>>,
    total_matches: Rc<RefCell<i32>>,
}

impl SearchBar {
    pub fn new() -> Self {
        let container = Box::builder()
            .orientation(Orientation::Horizontal)
            .spacing(6)
            .margin_top(6)
            .margin_bottom(6)
            .margin_start(6)
            .margin_end(6)
            .build();

        let search_entry = Entry::builder()
            .placeholder_text("Search...")
            .hexpand(true)
            .build();
        container.append(&search_entry);

        let prev_btn = Button::builder()
            .label("↑")
            .tooltip_text("Previous match")
            .build();
        container.append(&prev_btn);

        let next_btn = Button::builder()
            .label("↓")
            .tooltip_text("Next match")
            .build();
        container.append(&next_btn);

        let match_label = Label::new(Some(""));
        container.append(&match_label);

        let close_btn = Button::builder()
            .label("✕")
            .tooltip_text("Close search (Esc)")
            .build();
        container.append(&close_btn);

        let search_text = Rc::new(RefCell::new(String::new()));
        let current_match = Rc::new(RefCell::new(0));
        let total_matches = Rc::new(RefCell::new(0));

        Self {
            container,
            search_entry,
            prev_btn,
            next_btn,
            close_btn,
            match_label,
            search_text,
            current_match,
            total_matches,
        }
    }

    pub fn widget(&self) -> &Box {
        &self.container
    }

    pub fn show(&self) {
        self.container.set_visible(true);
        self.search_entry.grab_focus();
    }

    pub fn hide(&self) {
        self.container.set_visible(false);
    }

    pub fn is_visible(&self) -> bool {
        self.container.is_visible()
    }

    pub fn search_text(&self) -> String {
        self.search_text.borrow().clone()
    }

    pub fn set_search_text(&self, text: &str) {
        self.search_entry.set_text(text);
        *self.search_text.borrow_mut() = text.to_string();
    }

    pub fn current_match(&self) -> i32 {
        *self.current_match.borrow()
    }

    pub fn set_current_match(&self, idx: i32) {
        *self.current_match.borrow_mut() = idx;
        self.update_match_label();
    }

    pub fn total_matches(&self) -> i32 {
        *self.total_matches.borrow()
    }

    pub fn set_total_matches(&self, count: i32) {
        *self.total_matches.borrow_mut() = count;
        self.update_match_label();
    }

    fn update_match_label(&self) {
        let total = *self.total_matches.borrow();
        if total > 0 {
            let current = *self.current_match.borrow() + 1;
            self.match_label.set_text(&format!("{}/{} ", current, total));
        } else if !self.search_text.borrow().is_empty() {
            self.match_label.set_text("0/0 ");
        } else {
            self.match_label.set_text("");
        }
    }

    pub fn clear(&self) {
        self.search_entry.set_text("");
        *self.search_text.borrow_mut() = String::new();
        *self.current_match.borrow_mut() = 0;
        *self.total_matches.borrow_mut() = 0;
        self.match_label.set_text("");
    }

    pub fn connect_search_changed<F: Fn(&str) + 'static>(&self, callback: F) {
        self.search_entry.connect_changed(move |entry| {
            let text = entry.text().to_string();
            callback(&text);
        });
    }

    pub fn connect_prev_clicked<F: Fn() + 'static>(&self, callback: F) {
        self.prev_btn.connect_clicked(move |_| {
            callback();
        });
    }

    pub fn connect_next_clicked<F: Fn() + 'static>(&self, callback: F) {
        self.next_btn.connect_clicked(move |_| {
            callback();
        });
    }

    pub fn connect_close_clicked<F: Fn() + 'static>(&self, callback: F) {
        self.close_btn.connect_clicked(move |_| {
            callback();
        });
    }

    pub fn connect_activate<F: Fn() + 'static>(&self, callback: F) {
        self.search_entry.connect_activate(move |_| {
            callback();
        });
    }
}

impl Default for SearchBar {
    fn default() -> Self {
        Self::new()
    }
}

/// 搜索管理器
pub struct SearchManager {
    search_tag: Option<TextTag>,
    current_match_tag: Option<TextTag>,
    matches: Vec<(i32, i32)>,
}

impl SearchManager {
    pub fn new() -> Self {
        Self {
            search_tag: None,
            current_match_tag: None,
            matches: Vec::new(),
        }
    }

    pub fn setup_tags(&mut self, buffer: &TextBuffer) {
        let tag_table = buffer.tag_table();

        let search_tag = TextTag::builder()
            .name("search_highlight")
            .background("#FFFF00")
            .foreground("#000000")
            .build();
        tag_table.add(&search_tag);
        self.search_tag = Some(search_tag);

        let current_tag = TextTag::builder()
            .name("current_match")
            .background("#FF8800")
            .foreground("#FFFFFF")
            .weight(700)
            .build();
        tag_table.add(&current_tag);
        self.current_match_tag = Some(current_tag);
    }

    pub fn search(&mut self, buffer: &TextBuffer, search_text: &str, _case_sensitive: bool) -> i32 {
        self.clear_highlights(buffer);
        self.matches.clear();

        if search_text.is_empty() {
            return 0;
        }

        let text = buffer.text(&buffer.start_iter(), &buffer.end_iter(), true);
        let search_lower = search_text.to_lowercase();
        let text_lower = text.to_lowercase();
        
        let mut start_pos = 0;
        while let Some(pos) = text_lower[start_pos..].find(&search_lower) {
            let actual_pos = start_pos + pos;
            let end_pos = actual_pos + search_text.len();
            self.matches.push((actual_pos as i32, end_pos as i32));
            start_pos = end_pos;
        }

        if let Some(ref tag) = self.search_tag {
            for (start, end) in &self.matches {
                let start_iter = buffer.iter_at_offset(*start);
                let end_iter = buffer.iter_at_offset(*end);
                buffer.apply_tag(tag, &start_iter, &end_iter);
            }
        }

        self.matches.len() as i32
    }

    pub fn navigate_to_match(&mut self, buffer: &TextBuffer, index: i32) -> Option<TextIter> {
        if index < 0 || index >= self.matches.len() as i32 {
            return None;
        }

        if let Some(ref tag) = self.current_match_tag {
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            buffer.remove_tag(tag, &start, &end);
        }

        let (start_off, end_off) = self.matches[index as usize];
        
        if let Some(ref tag) = self.current_match_tag {
            let start_iter = buffer.iter_at_offset(start_off);
            let end_iter = buffer.iter_at_offset(end_off);
            buffer.apply_tag(tag, &start_iter, &end_iter);
        }

        Some(buffer.iter_at_offset(start_off))
    }

    pub fn clear_highlights(&self, buffer: &TextBuffer) {
        let start = buffer.start_iter();
        let end = buffer.end_iter();
        
        if let Some(ref tag) = self.search_tag {
            buffer.remove_tag(tag, &start, &end);
        }
        if let Some(ref tag) = self.current_match_tag {
            buffer.remove_tag(tag, &start, &end);
        }
    }

    pub fn match_count(&self) -> i32 {
        self.matches.len() as i32
    }

    pub fn clear(&mut self) {
        self.matches.clear();
    }
}