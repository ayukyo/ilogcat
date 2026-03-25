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
    ButtonNewTab,
    ButtonApply,
    ButtonAdvanced,
    ButtonAuto,
    ButtonLatest,
    ButtonExport,
    ButtonStats,
    ButtonImport,
    ButtonTheme,
    ButtonLanguage,
    ButtonResetDefault,

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
    SourceSshCommand,

    // 日志级别
    LevelTrace,
    LevelVerbose,
    LevelDebug,
    LevelInfo,
    LevelWarn,
    LevelError,
    LevelFatal,
    LevelCritical,

    // 状态
    StatusReady,
    StatusRunning,
    StatusPaused,
    StatusConnected,
    StatusDisconnected,
    StatusConnecting,

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
    DialogSettings,
    DialogStats,

    // 消息
    MsgSelectLogSource,
    MsgSettingsSaved,
    MsgCustomKeywordsUpdated,
    MsgNoSavedServers,
    MsgSuggestions,
    MsgRestartRequired,
    MsgFilterLogs,

    // 工具提示
    TooltipClearLogs,
    TooltipPauseResume,
    TooltipSettings,
    TooltipNewTab,
    TooltipAuto,
    TooltipLatest,
    TooltipExport,
    TooltipStats,
    TooltipTheme,
    TooltipLanguage,
    TooltipExportSettings,
    TooltipImportSettings,
    TooltipApplyFilter,
    TooltipClearFilter,
    TooltipAdvanced,
    
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
    ErrorFailedToStartDmesg,
    ErrorFailedToStartJournalctl,
    ErrorExportFailed,
    ErrorImportFailed,
    ErrorSaveFailed,

    // 信息消息
    InfoExportSuccessful,
    InfoImportSuccessful,
    InfoThemeChanged,
    InfoNoActiveTab,

    // 统计
    StatsTitle,
    StatsOverview,
    StatsTotalLogs,
    StatsFiltered,
    StatsRate,
    StatsUptime,
    StatsLevelDistribution,

    // 确认对话框
    ConfirmRestartRequired,
    ConfirmRestartMessage,

    // 标签页
    TabName,

    // 自定义关键字对话框
    CustomKeywordsInfo,
    CustomKeywordsPlaceholder,

    // 日志源选择对话框
    DialogSelectSource,
    SourceDmesgDesc,
    SourceJournalctlDesc,
    SourceFileDesc,

    // 命令输入
    PlaceholderCommand,
    ButtonRun,
    TooltipRunCommand,
    ErrorCommandFailed,

    // 高级过滤器对话框
    DialogAdvancedFilter,
    LabelCaseSensitive,
    LabelUseRegex,
    LabelInclude,
    LabelExclude,
    PlaceholderPattern,
    ButtonAdd,
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
        map.insert(I18nKey::ButtonNewTab, "+ New Tab");
        map.insert(I18nKey::ButtonApply, "Apply");
        map.insert(I18nKey::ButtonAdvanced, "Advanced");
        map.insert(I18nKey::ButtonAuto, "Auto");
        map.insert(I18nKey::ButtonLatest, "Latest");
        map.insert(I18nKey::ButtonExport, "Export");
        map.insert(I18nKey::ButtonStats, "Stats");
        map.insert(I18nKey::ButtonImport, "Import");
        map.insert(I18nKey::ButtonTheme, "Theme");
        map.insert(I18nKey::ButtonLanguage, "Language");
        map.insert(I18nKey::ButtonResetDefault, "Reset Default");

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
        map.insert(I18nKey::SourceSshCommand, "SSH Command...");

        // 日志级别
        map.insert(I18nKey::LevelTrace, "Trace");
        map.insert(I18nKey::LevelVerbose, "Verbose");
        map.insert(I18nKey::LevelDebug, "Debug");
        map.insert(I18nKey::LevelInfo, "Info");
        map.insert(I18nKey::LevelWarn, "Warn");
        map.insert(I18nKey::LevelError, "Error");
        map.insert(I18nKey::LevelFatal, "Fatal");
        map.insert(I18nKey::LevelCritical, "Critical");

        // 状态
        map.insert(I18nKey::StatusReady, "Ready - Select a log source to begin");
        map.insert(I18nKey::StatusRunning, "Running");
        map.insert(I18nKey::StatusPaused, "PAUSED");
        map.insert(I18nKey::StatusConnected, "Connected");
        map.insert(I18nKey::StatusDisconnected, "Disconnected");
        map.insert(I18nKey::StatusConnecting, "Connecting...");

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
        map.insert(I18nKey::DialogSettings, "Settings");
        map.insert(I18nKey::DialogStats, "Log Statistics");

        // 消息
        map.insert(I18nKey::MsgSelectLogSource, "Ready - Select a log source to begin");
        map.insert(I18nKey::MsgSettingsSaved, "Settings have been saved.");
        map.insert(I18nKey::MsgCustomKeywordsUpdated, "Custom level keywords have been updated.");
        map.insert(I18nKey::MsgNoSavedServers, "No saved servers");
        map.insert(I18nKey::MsgSuggestions, "Suggestions: dmesg -w, journalctl -f, tail -f /var/log/syslog");
        map.insert(I18nKey::MsgRestartRequired, "Restart required for language change to take full effect.");
        map.insert(I18nKey::MsgFilterLogs, "Filter logs (Enter to apply)...");

        // 工具提示
        map.insert(I18nKey::TooltipClearLogs, "Clear all logs (Ctrl+L)");
        map.insert(I18nKey::TooltipPauseResume, "Pause/Resume log stream (Ctrl+S)");
        map.insert(I18nKey::TooltipSettings, "Configure custom level keywords");
        map.insert(I18nKey::TooltipNewTab, "Create new log tab (Ctrl+T)");
        map.insert(I18nKey::TooltipAuto, "Toggle auto-scroll (enabled by default, pauses when scrolling up)");
        map.insert(I18nKey::TooltipLatest, "Jump to latest log and resume auto-scroll");
        map.insert(I18nKey::TooltipExport, "Export logs to file (Ctrl+Shift+E)");
        map.insert(I18nKey::TooltipStats, "Show log statistics");
        map.insert(I18nKey::TooltipTheme, "Change application theme (Light/Dark)");
        map.insert(I18nKey::TooltipLanguage, "Change application language");
        map.insert(I18nKey::TooltipExportSettings, "Export all settings to a file");
        map.insert(I18nKey::TooltipImportSettings, "Import settings from a file");
        map.insert(I18nKey::TooltipApplyFilter, "Apply filter to current tab");
        map.insert(I18nKey::TooltipClearFilter, "Clear all filters");
        map.insert(I18nKey::TooltipAdvanced, "Open advanced filter dialog (Ctrl+Shift+F)");
        
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
        map.insert(I18nKey::ErrorFailedToStartDmesg, "Failed to Start dmesg");
        map.insert(I18nKey::ErrorFailedToStartJournalctl, "Failed to Start journalctl");
        map.insert(I18nKey::ErrorExportFailed, "Export Failed");
        map.insert(I18nKey::ErrorImportFailed, "Import Failed");
        map.insert(I18nKey::ErrorSaveFailed, "Save Failed");

        // 信息消息
        map.insert(I18nKey::InfoExportSuccessful, "Export Successful");
        map.insert(I18nKey::InfoImportSuccessful, "Import Successful");
        map.insert(I18nKey::InfoThemeChanged, "Theme setting has been applied.");
        map.insert(I18nKey::InfoNoActiveTab, "Please open a log tab first.");

        // 统计
        map.insert(I18nKey::StatsTitle, "Log Statistics");
        map.insert(I18nKey::StatsOverview, "Overview");
        map.insert(I18nKey::StatsTotalLogs, "Total Logs:");
        map.insert(I18nKey::StatsFiltered, "Filtered:");
        map.insert(I18nKey::StatsRate, "Rate:");
        map.insert(I18nKey::StatsUptime, "Uptime:");
        map.insert(I18nKey::StatsLevelDistribution, "Level Distribution");

        // 确认对话框
        map.insert(I18nKey::ConfirmRestartRequired, "Restart Required");
        map.insert(I18nKey::ConfirmRestartMessage, "Language has been changed. The application needs to restart for the change to take full effect.\n\nDo you want to restart now?");

        // 标签页
        map.insert(I18nKey::TabName, "Log");

        // 自定义关键字对话框
        map.insert(I18nKey::CustomKeywordsInfo, "Define custom keywords to detect log levels.\nKeywords are case-insensitive.");
        map.insert(I18nKey::CustomKeywordsPlaceholder, "e.g., [v], [verbose]");

        // 日志源选择对话框
        map.insert(I18nKey::DialogSelectSource, "Select Log Source");
        map.insert(I18nKey::SourceDmesgDesc, "Kernel ring buffer messages");
        map.insert(I18nKey::SourceJournalctlDesc, "Systemd journal logs");
        map.insert(I18nKey::SourceFileDesc, "Watch a log file");

        // 命令输入
        map.insert(I18nKey::PlaceholderCommand, "Enter command (e.g., dmesg -w, tail -f /var/log/syslog)");
        map.insert(I18nKey::ButtonRun, "Run");
        map.insert(I18nKey::TooltipRunCommand, "Execute command and display output");
        map.insert(I18nKey::ErrorCommandFailed, "Command execution failed");

        // 高级过滤器对话框
        map.insert(I18nKey::DialogAdvancedFilter, "Advanced Filter");
        map.insert(I18nKey::LabelCaseSensitive, "Case sensitive");
        map.insert(I18nKey::LabelUseRegex, "Use regex");
        map.insert(I18nKey::LabelInclude, "Include");
        map.insert(I18nKey::LabelExclude, "Exclude");
        map.insert(I18nKey::PlaceholderPattern, "Enter pattern...");
        map.insert(I18nKey::ButtonAdd, "Add");

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
        map.insert(I18nKey::ButtonNewTab, "+ 新建标签");
        map.insert(I18nKey::ButtonApply, "应用");
        map.insert(I18nKey::ButtonAdvanced, "高级");
        map.insert(I18nKey::ButtonAuto, "自动");
        map.insert(I18nKey::ButtonLatest, "最新");
        map.insert(I18nKey::ButtonExport, "导出");
        map.insert(I18nKey::ButtonStats, "统计");
        map.insert(I18nKey::ButtonImport, "导入");
        map.insert(I18nKey::ButtonTheme, "主题");
        map.insert(I18nKey::ButtonLanguage, "语言");
        map.insert(I18nKey::ButtonResetDefault, "恢复默认");

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
        map.insert(I18nKey::SourceSshCommand, "SSH 命令...");

        // 日志级别
        map.insert(I18nKey::LevelTrace, "跟踪");
        map.insert(I18nKey::LevelVerbose, "详细");
        map.insert(I18nKey::LevelDebug, "调试");
        map.insert(I18nKey::LevelInfo, "信息");
        map.insert(I18nKey::LevelWarn, "警告");
        map.insert(I18nKey::LevelError, "错误");
        map.insert(I18nKey::LevelFatal, "致命");
        map.insert(I18nKey::LevelCritical, "严重");

        // 状态
        map.insert(I18nKey::StatusReady, "就绪 - 请选择日志源开始");
        map.insert(I18nKey::StatusRunning, "运行中");
        map.insert(I18nKey::StatusPaused, "已暂停");
        map.insert(I18nKey::StatusConnected, "已连接");
        map.insert(I18nKey::StatusDisconnected, "已断开");
        map.insert(I18nKey::StatusConnecting, "正在连接...");

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
        map.insert(I18nKey::DialogSettings, "设置");
        map.insert(I18nKey::DialogStats, "日志统计");

        // 消息
        map.insert(I18nKey::MsgSelectLogSource, "就绪 - 请选择日志源开始");
        map.insert(I18nKey::MsgSettingsSaved, "设置已保存。");
        map.insert(I18nKey::MsgCustomKeywordsUpdated, "自定义级别关键字已更新。");
        map.insert(I18nKey::MsgNoSavedServers, "没有保存的服务器");
        map.insert(I18nKey::MsgSuggestions, "建议命令: dmesg -w, journalctl -f, tail -f /var/log/syslog");
        map.insert(I18nKey::MsgRestartRequired, "需要重启以使语言更改完全生效。");
        map.insert(I18nKey::MsgFilterLogs, "过滤日志 (回车应用)...");

        // 工具提示
        map.insert(I18nKey::TooltipClearLogs, "清空所有日志 (Ctrl+L)");
        map.insert(I18nKey::TooltipPauseResume, "暂停/继续日志流 (Ctrl+S)");
        map.insert(I18nKey::TooltipSettings, "配置自定义级别关键字");
        map.insert(I18nKey::TooltipNewTab, "创建新日志标签 (Ctrl+T)");
        map.insert(I18nKey::TooltipAuto, "切换自动滚动 (默认启用, 向上滚动时暂停)");
        map.insert(I18nKey::TooltipLatest, "跳转到最新日志并恢复自动滚动");
        map.insert(I18nKey::TooltipExport, "导出日志到文件 (Ctrl+Shift+E)");
        map.insert(I18nKey::TooltipStats, "显示日志统计");
        map.insert(I18nKey::TooltipTheme, "更改应用主题 (浅色/深色)");
        map.insert(I18nKey::TooltipLanguage, "更改应用语言");
        map.insert(I18nKey::TooltipExportSettings, "导出所有设置到文件");
        map.insert(I18nKey::TooltipImportSettings, "从文件导入设置");
        map.insert(I18nKey::TooltipApplyFilter, "应用过滤到当前标签");
        map.insert(I18nKey::TooltipClearFilter, "清除所有过滤器");
        map.insert(I18nKey::TooltipAdvanced, "打开高级过滤对话框 (Ctrl+Shift+F)");
        
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
        map.insert(I18nKey::ErrorFailedToStartDmesg, "启动 dmesg 失败");
        map.insert(I18nKey::ErrorFailedToStartJournalctl, "启动 journalctl 失败");
        map.insert(I18nKey::ErrorExportFailed, "导出失败");
        map.insert(I18nKey::ErrorImportFailed, "导入失败");
        map.insert(I18nKey::ErrorSaveFailed, "保存失败");

        // 信息消息
        map.insert(I18nKey::InfoExportSuccessful, "导出成功");
        map.insert(I18nKey::InfoImportSuccessful, "导入成功");
        map.insert(I18nKey::InfoThemeChanged, "主题设置已应用。");
        map.insert(I18nKey::InfoNoActiveTab, "请先打开一个日志标签页。");

        // 统计
        map.insert(I18nKey::StatsTitle, "日志统计");
        map.insert(I18nKey::StatsOverview, "概览");
        map.insert(I18nKey::StatsTotalLogs, "总日志数:");
        map.insert(I18nKey::StatsFiltered, "已过滤:");
        map.insert(I18nKey::StatsRate, "速率:");
        map.insert(I18nKey::StatsUptime, "运行时间:");
        map.insert(I18nKey::StatsLevelDistribution, "级别分布");

        // 确认对话框
        map.insert(I18nKey::ConfirmRestartRequired, "需要重启");
        map.insert(I18nKey::ConfirmRestartMessage, "语言已更改。应用程序需要重启才能完全生效。\n\n是否现在重启？");

        // 标签页
        map.insert(I18nKey::TabName, "日志");

        // 自定义关键字对话框
        map.insert(I18nKey::CustomKeywordsInfo, "定义自定义关键字来检测日志级别。\n关键字不区分大小写。");
        map.insert(I18nKey::CustomKeywordsPlaceholder, "例如：[v], [verbose]");

        // 日志源选择对话框
        map.insert(I18nKey::DialogSelectSource, "选择日志源");
        map.insert(I18nKey::SourceDmesgDesc, "内核环形缓冲区消息");
        map.insert(I18nKey::SourceJournalctlDesc, "Systemd 日志");
        map.insert(I18nKey::SourceFileDesc, "监视日志文件");

        // 命令输入
        map.insert(I18nKey::PlaceholderCommand, "输入命令 (如: dmesg -w, tail -f /var/log/syslog)");
        map.insert(I18nKey::ButtonRun, "执行");
        map.insert(I18nKey::TooltipRunCommand, "执行命令并显示输出");
        map.insert(I18nKey::ErrorCommandFailed, "命令执行失败");

        // 高级过滤器对话框
        map.insert(I18nKey::DialogAdvancedFilter, "高级过滤");
        map.insert(I18nKey::LabelCaseSensitive, "区分大小写");
        map.insert(I18nKey::LabelUseRegex, "使用正则");
        map.insert(I18nKey::LabelInclude, "包含");
        map.insert(I18nKey::LabelExclude, "排除");
        map.insert(I18nKey::PlaceholderPattern, "输入模式...");
        map.insert(I18nKey::ButtonAdd, "添加");

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