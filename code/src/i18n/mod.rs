//! 国际化 (i18n) 支持模块
//!
//! 提供多语言支持，目前支持中文和英文

use std::collections::HashMap;
use std::sync::OnceLock;

/// 支持的语言
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    #[default]
    English,
    Chinese,
}

impl Language {
    /// 从字符串解析语言
    pub fn from_str(s: &str) -> Self {
        match s.to_lowercase().as_str() {
            "zh" | "zh-cn" | "chinese" | "中文" => Language::Chinese,
            _ => Language::English,
        }
    }

    /// 转换为字符串
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::English => "en",
            Language::Chinese => "zh",
        }
    }

    /// 获取显示名称
    pub fn display_name(&self) -> &'static str {
        match self {
            Language::English => "English",
            Language::Chinese => "中文",
        }
    }
}

/// 翻译键
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum I18nKey {
    // 通用
    AppName,
    AppTitle,
    
    // 按钮
    ButtonOk,
    ButtonCancel,
    ButtonSave,
    ButtonClear,
    ButtonPause,
    ButtonResume,
    ButtonConnect,
    ButtonExecute,
    ButtonSettings,
    
    // 标签
    LabelSource,
    LabelMinLevel,
    LabelFilter,
    LabelName,
    LabelHost,
    LabelPort,
    LabelUsername,
    LabelPassword,
    LabelCommand,
    LabelServer,
    LabelTheme,
    LabelLanguage,
    
    // 日志源
    SourceDmesg,
    SourceJournalctl,
    SourceFile,
    SourceSsh,
    
    // 日志级别
    LevelVerbose,
    LevelDebug,
    LevelInfo,
    LevelWarn,
    LevelError,
    LevelFatal,
    
    // 状态
    StatusReady,
    StatusRunning,
    StatusPaused,
    StatusConnected,
    StatusDisconnected,
    
    // 对话框标题
    DialogSshConnection,
    DialogSshCommand,
    DialogCustomKeywords,
    DialogThemeSettings,
    DialogLanguageSettings,
    DialogExportSettings,
    DialogImportSettings,
    DialogError,
    DialogInfo,
    DialogConfirm,
    
    // 消息
    MsgSelectLogSource,
    MsgSettingsSaved,
    MsgCustomKeywordsUpdated,
    MsgNoSavedServers,
    MsgSuggestions,
    
    // 工具提示
    TooltipClearLogs,
    TooltipPauseResume,
    TooltipSettings,
    
    // 主题
    ThemeLight,
    ThemeDark,
    
    // 菜单
    MenuFile,
    MenuEdit,
    MenuView,
    MenuSettings,
    MenuHelp,
    
    // 错误消息
    ErrorFailedToStart,
    ErrorSshConnection,
}

/// 翻译管理器
pub struct I18n {
    current_lang: Language,
    translations: HashMap<I18nKey, &'static str>,
}

impl I18n {
    /// 创建新的翻译管理器
    pub fn new(lang: Language) -> Self {
        let mut i18n = Self {
            current_lang: lang,
            translations: HashMap::new(),
        };
        i18n.load_translations();
        i18n
    }

    /// 获取当前语言
    pub fn current_language(&self) -> Language {
        self.current_lang
    }

    /// 切换语言
    pub fn set_language(&mut self, lang: Language) {
        self.current_lang = lang;
        self.translations.clear();
        self.load_translations();
    }

    /// 获取翻译文本
    pub fn t(&self, key: I18nKey) -> &str {
        self.translations.get(&key).copied().unwrap_or("???")
    }

    /// 加载翻译
    fn load_translations(&mut self) {
        let translations: HashMap<I18nKey, &'static str> = match self.current_lang {
            Language::Chinese => Self::chinese_translations(),
            Language::English => Self::english_translations(),
        };
        self.translations = translations;
    }

    /// 英文翻译
    fn english_translations() -> HashMap<I18nKey, &'static str> {
        let mut map = HashMap::new();
        
        // 通用
        map.insert(I18nKey::AppName, "iLogCat");
        map.insert(I18nKey::AppTitle, "iLogCat - Linux Log Viewer");
        
        // 按钮
        map.insert(I18nKey::ButtonOk, "OK");
        map.insert(I18nKey::ButtonCancel, "Cancel");
        map.insert(I18nKey::ButtonSave, "Save");
        map.insert(I18nKey::ButtonClear, "Clear");
        map.insert(I18nKey::ButtonPause, "Pause");
        map.insert(I18nKey::ButtonResume, "Resume");
        map.insert(I18nKey::ButtonConnect, "Connect");
        map.insert(I18nKey::ButtonExecute, "Execute");
        map.insert(I18nKey::ButtonSettings, "Settings");
        
        // 标签
        map.insert(I18nKey::LabelSource, "Source:");
        map.insert(I18nKey::LabelMinLevel, "Min Level:");
        map.insert(I18nKey::LabelFilter, "Filter:");
        map.insert(I18nKey::LabelName, "Name:");
        map.insert(I18nKey::LabelHost, "Host:");
        map.insert(I18nKey::LabelPort, "Port:");
        map.insert(I18nKey::LabelUsername, "Username:");
        map.insert(I18nKey::LabelPassword, "Password:");
        map.insert(I18nKey::LabelCommand, "Command:");
        map.insert(I18nKey::LabelServer, "Server:");
        map.insert(I18nKey::LabelTheme, "Theme:");
        map.insert(I18nKey::LabelLanguage, "Language:");
        
        // 日志源
        map.insert(I18nKey::SourceDmesg, "Local: dmesg");
        map.insert(I18nKey::SourceJournalctl, "Local: journalctl");
        map.insert(I18nKey::SourceFile, "File...");
        map.insert(I18nKey::SourceSsh, "SSH...");
        
        // 日志级别
        map.insert(I18nKey::LevelVerbose, "Verbose");
        map.insert(I18nKey::LevelDebug, "Debug");
        map.insert(I18nKey::LevelInfo, "Info");
        map.insert(I18nKey::LevelWarn, "Warn");
        map.insert(I18nKey::LevelError, "Error");
        map.insert(I18nKey::LevelFatal, "Fatal");
        
        // 状态
        map.insert(I18nKey::StatusReady, "Ready - Select a log source to begin");
        map.insert(I18nKey::StatusRunning, "Running");
        map.insert(I18nKey::StatusPaused, "PAUSED");
        map.insert(I18nKey::StatusConnected, "Connected");
        map.insert(I18nKey::StatusDisconnected, "Disconnected");
        
        // 对话框标题
        map.insert(I18nKey::DialogSshConnection, "SSH Connection");
        map.insert(I18nKey::DialogSshCommand, "Execute SSH Command");
        map.insert(I18nKey::DialogCustomKeywords, "Custom Level Keywords");
        map.insert(I18nKey::DialogThemeSettings, "Theme Settings");
        map.insert(I18nKey::DialogLanguageSettings, "Language Settings");
        map.insert(I18nKey::DialogExportSettings, "Export Settings");
        map.insert(I18nKey::DialogImportSettings, "Import Settings");
        map.insert(I18nKey::DialogError, "Error");
        map.insert(I18nKey::DialogInfo, "Information");
        map.insert(I18nKey::DialogConfirm, "Confirm");
        
        // 消息
        map.insert(I18nKey::MsgSelectLogSource, "Ready - Select a log source to begin");
        map.insert(I18nKey::MsgSettingsSaved, "Settings have been saved.");
        map.insert(I18nKey::MsgCustomKeywordsUpdated, "Custom level keywords have been updated.");
        map.insert(I18nKey::MsgNoSavedServers, "No saved servers");
        map.insert(I18nKey::MsgSuggestions, "Suggestions");
        
        // 工具提示
        map.insert(I18nKey::TooltipClearLogs, "Clear all logs (Ctrl+L)");
        map.insert(I18nKey::TooltipPauseResume, "Pause/Resume log stream (Ctrl+S)");
        map.insert(I18nKey::TooltipSettings, "Configure custom level keywords");
        
        // 主题
        map.insert(I18nKey::ThemeLight, "Light Theme");
        map.insert(I18nKey::ThemeDark, "Dark Theme");
        
        // 菜单
        map.insert(I18nKey::MenuFile, "File");
        map.insert(I18nKey::MenuEdit, "Edit");
        map.insert(I18nKey::MenuView, "View");
        map.insert(I18nKey::MenuSettings, "Settings");
        map.insert(I18nKey::MenuHelp, "Help");
        
        // 错误消息
        map.insert(I18nKey::ErrorFailedToStart, "Failed to start");
        map.insert(I18nKey::ErrorSshConnection, "SSH connection failed");
        
        map
    }

    /// 中文翻译
    fn chinese_translations() -> HashMap<I18nKey, &'static str> {
        let mut map = HashMap::new();
        
        // 通用
        map.insert(I18nKey::AppName, "iLogCat");
        map.insert(I18nKey::AppTitle, "iLogCat - Linux 日志查看器");
        
        // 按钮
        map.insert(I18nKey::ButtonOk, "确定");
        map.insert(I18nKey::ButtonCancel, "取消");
        map.insert(I18nKey::ButtonSave, "保存");
        map.insert(I18nKey::ButtonClear, "清空");
        map.insert(I18nKey::ButtonPause, "暂停");
        map.insert(I18nKey::ButtonResume, "继续");
        map.insert(I18nKey::ButtonConnect, "连接");
        map.insert(I18nKey::ButtonExecute, "执行");
        map.insert(I18nKey::ButtonSettings, "设置");
        
        // 标签
        map.insert(I18nKey::LabelSource, "日志源:");
        map.insert(I18nKey::LabelMinLevel, "最低级别:");
        map.insert(I18nKey::LabelFilter, "过滤:");
        map.insert(I18nKey::LabelName, "名称:");
        map.insert(I18nKey::LabelHost, "主机:");
        map.insert(I18nKey::LabelPort, "端口:");
        map.insert(I18nKey::LabelUsername, "用户名:");
        map.insert(I18nKey::LabelPassword, "密码:");
        map.insert(I18nKey::LabelCommand, "命令:");
        map.insert(I18nKey::LabelServer, "服务器:");
        map.insert(I18nKey::LabelTheme, "主题:");
        map.insert(I18nKey::LabelLanguage, "语言:");
        
        // 日志源
        map.insert(I18nKey::SourceDmesg, "本地: dmesg");
        map.insert(I18nKey::SourceJournalctl, "本地: journalctl");
        map.insert(I18nKey::SourceFile, "文件...");
        map.insert(I18nKey::SourceSsh, "SSH...");
        
        // 日志级别
        map.insert(I18nKey::LevelVerbose, "详细");
        map.insert(I18nKey::LevelDebug, "调试");
        map.insert(I18nKey::LevelInfo, "信息");
        map.insert(I18nKey::LevelWarn, "警告");
        map.insert(I18nKey::LevelError, "错误");
        map.insert(I18nKey::LevelFatal, "致命");
        
        // 状态
        map.insert(I18nKey::StatusReady, "就绪 - 请选择日志源开始");
        map.insert(I18nKey::StatusRunning, "运行中");
        map.insert(I18nKey::StatusPaused, "已暂停");
        map.insert(I18nKey::StatusConnected, "已连接");
        map.insert(I18nKey::StatusDisconnected, "已断开");
        
        // 对话框标题
        map.insert(I18nKey::DialogSshConnection, "SSH 连接");
        map.insert(I18nKey::DialogSshCommand, "执行 SSH 命令");
        map.insert(I18nKey::DialogCustomKeywords, "自定义级别关键字");
        map.insert(I18nKey::DialogThemeSettings, "主题设置");
        map.insert(I18nKey::DialogLanguageSettings, "语言设置");
        map.insert(I18nKey::DialogExportSettings, "导出设置");
        map.insert(I18nKey::DialogImportSettings, "导入设置");
        map.insert(I18nKey::DialogError, "错误");
        map.insert(I18nKey::DialogInfo, "信息");
        map.insert(I18nKey::DialogConfirm, "确认");
        
        // 消息
        map.insert(I18nKey::MsgSelectLogSource, "就绪 - 请选择日志源开始");
        map.insert(I18nKey::MsgSettingsSaved, "设置已保存。");
        map.insert(I18nKey::MsgCustomKeywordsUpdated, "自定义级别关键字已更新。");
        map.insert(I18nKey::MsgNoSavedServers, "没有保存的服务器");
        map.insert(I18nKey::MsgSuggestions, "建议");
        
        // 工具提示
        map.insert(I18nKey::TooltipClearLogs, "清空所有日志 (Ctrl+L)");
        map.insert(I18nKey::TooltipPauseResume, "暂停/继续日志流 (Ctrl+S)");
        map.insert(I18nKey::TooltipSettings, "配置自定义级别关键字");
        
        // 主题
        map.insert(I18nKey::ThemeLight, "浅色主题");
        map.insert(I18nKey::ThemeDark, "深色主题");
        
        // 菜单
        map.insert(I18nKey::MenuFile, "文件");
        map.insert(I18nKey::MenuEdit, "编辑");
        map.insert(I18nKey::MenuView, "视图");
        map.insert(I18nKey::MenuSettings, "设置");
        map.insert(I18nKey::MenuHelp, "帮助");
        
        // 错误消息
        map.insert(I18nKey::ErrorFailedToStart, "启动失败");
        map.insert(I18nKey::ErrorSshConnection, "SSH 连接失败");
        
        map
    }
}

// 全局 I18n 实例
static I18N: OnceLock<std::sync::Mutex<I18n>> = OnceLock::new();

/// 初始化全局 I18n
pub fn init(lang: Language) {
    let _ = I18N.set(std::sync::Mutex::new(I18n::new(lang)));
}

/// 获取全局 I18n 实例
pub fn i18n() -> std::sync::MutexGuard<'static, I18n> {
    I18N.get()
        .expect("I18n not initialized")
        .lock()
        .expect("Failed to lock I18n")
}

/// 翻译快捷函数
pub fn t(key: I18nKey) -> String {
    i18n().t(key).to_string()
}

/// 切换语言
pub fn set_language(lang: Language) {
    i18n().set_language(lang);
}

/// 获取当前语言
pub fn current_language() -> Language {
    i18n().current_language()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_language_from_str() {
        assert_eq!(Language::from_str("en"), Language::English);
        assert_eq!(Language::from_str("zh"), Language::Chinese);
        assert_eq!(Language::from_str("zh-cn"), Language::Chinese);
        assert_eq!(Language::from_str("Chinese"), Language::Chinese);
        assert_eq!(Language::from_str("中文"), Language::Chinese);
    }

    #[test]
    fn test_i18n_basic() {
        let mut i18n = I18n::new(Language::English);
        assert_eq!(i18n.t(I18nKey::ButtonOk), "OK");
        
        i18n.set_language(Language::Chinese);
        assert_eq!(i18n.t(I18nKey::ButtonOk), "确定");
    }
}