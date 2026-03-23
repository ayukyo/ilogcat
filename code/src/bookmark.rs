use gtk4::prelude::*;
use gtk4::{TextBuffer, TextIter, TextMark, TextTag, TextTagTable};
use std::cell::RefCell;
use std::rc::Rc;
use std::collections::HashMap;

/// 书签信息
#[derive(Debug, Clone)]
pub struct Bookmark {
    pub id: usize,
    pub line_number: i32,
    pub text: String,
    pub note: Option<String>,
}

/// 书签管理器
pub struct BookmarkManager {
    bookmarks: HashMap<usize, Bookmark>,
    next_id: usize,
    tag: Option<TextTag>,
}

impl BookmarkManager {
    pub fn new() -> Self {
        Self {
            bookmarks: HashMap::new(),
            next_id: 1,
            tag: None,
        }
    }

    /// 初始化书签标签样式
    pub fn init_tag(&mut self, buffer: &TextBuffer) {
        let tag_table = buffer.tag_table();

        // 创建书签标签 - 黄色背景高亮
        let bookmark_tag = TextTag::new(Some("bookmark"));
        bookmark_tag.set_background(Some("#FFD700"));
        bookmark_tag.set_foreground(Some("#000000"));
        bookmark_tag.set_weight(gtk4::pango::Weight::Bold.value());

        tag_table.add(&bookmark_tag);
        self.tag = Some(bookmark_tag);
    }

    /// 在指定行添加书签
    pub fn add_bookmark(&mut self, buffer: &TextBuffer, line_number: i32, note: Option<String>) -> Option<usize> {
        // 获取行内容
        let start_iter = buffer.iter_at_line(line_number)?;
        let end_iter = if let Some(next_line) = buffer.iter_at_line(line_number + 1) {
            next_line
        } else {
            buffer.end_iter()
        };

        let text = buffer.text(&start_iter, &end_iter, false).to_string();
        let text = text.trim().to_string();

        if text.is_empty() {
            return None;
        }

        // 创建书签
        let id = self.next_id;
        self.next_id += 1;

        let bookmark = Bookmark {
            id,
            line_number,
            text: text.clone(),
            note,
        };

        self.bookmarks.insert(id, bookmark);

        // 应用书签样式
        if let Some(ref tag) = self.tag {
            buffer.apply_tag(tag, &start_iter, &end_iter);
        }

        Some(id)
    }

    /// 移除书签
    pub fn remove_bookmark(&mut self, buffer: &TextBuffer, id: usize) -> bool {
        if let Some(bookmark) = self.bookmarks.remove(&id) {
            // 移除样式
            if let Some(start_iter) = buffer.iter_at_line(bookmark.line_number) {
                let end_iter = if let Some(next_line) = buffer.iter_at_line(bookmark.line_number + 1) {
                    next_line
                } else {
                    buffer.end_iter()
                };

                if let Some(ref tag) = self.tag {
                    buffer.remove_tag(tag, &start_iter, &end_iter);
                }
            }
            true
        } else {
            false
        }
    }

    /// 获取所有书签
    pub fn get_bookmarks(&self) -> Vec<&Bookmark> {
        let mut bookmarks: Vec<_> = self.bookmarks.values().collect();
        bookmarks.sort_by_key(|b| b.line_number);
        bookmarks
    }

    /// 获取书签数量
    pub fn count(&self) -> usize {
        self.bookmarks.len()
    }

    /// 跳转到指定书签
    pub fn goto_bookmark(&self, buffer: &TextBuffer, id: usize) -> Option<TextIter> {
        self.bookmarks.get(&id).and_then(|bookmark| {
            buffer.iter_at_line(bookmark.line_number)
        })
    }

    /// 清除所有书签
    pub fn clear_all(&mut self, buffer: &TextBuffer) {
        if let Some(ref tag) = self.tag {
            let start = buffer.start_iter();
            let end = buffer.end_iter();
            buffer.remove_tag(tag, &start, &end);
        }
        self.bookmarks.clear();
    }

    /// 导出书签列表
    pub fn export_bookmarks(&self) -> String {
        let mut output = String::new();
        output.push_str("# iLogCat Bookmarks\n\n");

        let mut bookmarks: Vec<_> = self.bookmarks.values().collect();
        bookmarks.sort_by_key(|b| b.line_number);

        for bookmark in bookmarks {
            output.push_str(&format!("## Line {}\n", bookmark.line_number));
            if let Some(ref note) = bookmark.note {
                output.push_str(&format!("Note: {}\n", note));
            }
            output.push_str(&format!("Content: {}\n\n", bookmark.text));
        }

        output
    }
}

/// 书签对话框
pub struct BookmarkDialog;

impl BookmarkDialog {
    /// 显示添加书签对话框
    pub fn show_add<F: Fn(Option<String>) + 'static>(
        parent: &gtk4::Window,
        line_text: &str,
        callback: F,
    ) {
        let dialog = gtk4::Dialog::new();
        dialog.set_title(Some("Add Bookmark"));
        dialog.set_transient_for(Some(parent));
        dialog.set_modal(true);
        dialog.set_default_size(400, 200);

        let content = dialog.content_area();
        content.set_spacing(12);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);

        // 显示行内容
        let line_label = gtk4::Label::new(Some(line_text));
        line_label.set_wrap(true);
        line_label.set_xalign(0.0);
        line_label.add_css_class("dim-label");
        content.append(&line_label);

        // 备注输入
        let note_label = gtk4::Label::new(Some("Note (optional):"));
        note_label.set_xalign(0.0);
        content.append(&note_label);

        let note_entry = gtk4::Entry::new();
        note_entry.set_placeholder_text(Some("Enter a note for this bookmark..."));
        content.append(&note_entry);

        dialog.add_button("Cancel", gtk4::ResponseType::Cancel);
        dialog.add_button("Add", gtk4::ResponseType::Accept);

        dialog.connect_response(move |dialog, response| {
            if response == gtk4::ResponseType::Accept {
                let note = note_entry.text();
                let note = if note.is_empty() {
                    None
                } else {
                    Some(note.to_string())
                };
                callback(note);
            }
            dialog.close();
        });

        dialog.show();
    }

    /// 显示书签列表对话框
    pub fn show_list<F: Fn(usize, bool) + Clone + 'static>(
        parent: &gtk4::Window,
        bookmarks: Vec<&Bookmark>,
        callback: F,
    ) {
        let dialog = gtk4::Dialog::new();
        dialog.set_title(Some("Bookmarks"));
        dialog.set_transient_for(Some(parent));
        dialog.set_modal(true);
        dialog.set_default_size(500, 400);

        let content = dialog.content_area();
        content.set_spacing(6);
        content.set_margin_top(12);
        content.set_margin_bottom(12);
        content.set_margin_start(12);
        content.set_margin_end(12);

        if bookmarks.is_empty() {
            let empty_label = gtk4::Label::new(Some("No bookmarks yet"));
            empty_label.add_css_class("dim-label");
            content.append(&empty_label);
        } else {
            let scrolled = gtk4::ScrolledWindow::new();
            scrolled.set_vexpand(true);

            let list_box = gtk4::ListBox::new();
            list_box.set_selection_mode(gtk4::SelectionMode::None);

            for bookmark in bookmarks {
                let row = gtk4::Box::new(gtk4::Orientation::Horizontal, 6);
                row.set_margin_top(6);
                row.set_margin_bottom(6);
                row.set_margin_start(6);
                row.set_margin_end(6);

                // 行号
                let line_label = gtk4::Label::new(Some(&format!("Line {}", bookmark.line_number)));
                line_label.set_width_chars(8);
                row.append(&line_label);

                // 内容
                let text = if bookmark.text.len() > 50 {
                    format!("{}...", &bookmark.text[..50])
                } else {
                    bookmark.text.clone()
                };
                let text_label = gtk4::Label::new(Some(&text));
                text_label.set_hexpand(true);
                text_label.set_xalign(0.0);
                row.append(&text_label);

                // 跳转按钮
                let goto_btn = gtk4::Button::with_label("Go");
                let id = bookmark.id;
                let cb = callback.clone();
                goto_btn.connect_clicked(move |_| {
                    cb(id, false);
                });
                row.append(&goto_btn);

                // 删除按钮
                let del_btn = gtk4::Button::from_icon_name("user-trash-symbolic");
                del_btn.add_css_class("flat");
                let cb = callback.clone();
                del_btn.connect_clicked(move |_| {
                    cb(id, true);
                });
                row.append(&del_btn);

                list_box.append(&row);
            }

            scrolled.set_child(Some(&list_box));
            content.append(&scrolled);
        }

        dialog.add_button("Close", gtk4::ResponseType::Close);
        dialog.connect_response(|dialog, _| {
            dialog.close();
        });

        dialog.show();
    }
}
