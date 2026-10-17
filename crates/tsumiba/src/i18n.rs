//! User-visible strings, in English and Japanese.
//!
//! Every string lives in [`Strings`], so a missing translation is a compile
//! error rather than a blank label.

use tsumiba_core::config::Language;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Ja,
}

/// The language to show, given the setting and the system locale.
pub fn resolve(setting: Language) -> Lang {
    match setting {
        Language::En => Lang::En,
        Language::Ja => Lang::Ja,
        Language::Auto => from_locale(sys_locale::get_locale().as_deref()),
    }
}

pub fn from_locale(locale: Option<&str>) -> Lang {
    match locale {
        Some(l) if l.to_ascii_lowercase().starts_with("ja") => Lang::Ja,
        _ => Lang::En,
    }
}

pub fn strings(lang: Lang) -> &'static Strings {
    match lang {
        Lang::En => &EN,
        Lang::Ja => &JA,
    }
}

pub struct Strings {
    // header
    pub search_hint: &'static str,
    pub keep_open: &'static str,
    pub settings: &'static str,
    pub hide: &'static str,
    // notes
    pub notes: &'static str,
    pub new_note: &'static str,
    pub untitled: &'static str,
    pub notes_empty: &'static str,
    pub note_hint: &'static str,
    pub pin: &'static str,
    pub unpin: &'static str,
    pub copy_text: &'static str,
    pub delete: &'static str,
    pub no_matches: &'static str,
    // files
    pub files: &'static str,
    pub add_files: &'static str,
    pub files_empty: &'static str,
    pub drop_to_add: &'static str,
    pub open: &'static str,
    pub show_in_folder: &'static str,
    pub copy_file: &'static str,
    pub copy_path: &'static str,
    pub save_copy: &'static str,
    pub rename: &'static str,
    pub remove: &'static str,
    pub missing: &'static str,
    pub linked: &'static str,
    pub folder: &'static str,
    pub from: &'static str,
    pub added: &'static str,
    // toasts
    pub undo: &'static str,
    pub cancel: &'static str,
    pub copied: &'static str,
    pub note_deleted: &'static str,
    pub file_removed: &'static str,
    // settings
    pub back: &'static str,
    pub general: &'static str,
    pub language: &'static str,
    pub language_auto: &'static str,
    pub theme: &'static str,
    pub theme_system: &'static str,
    pub theme_light: &'static str,
    pub theme_dark: &'static str,
    pub launch_at_login: &'static str,
    pub panel: &'static str,
    pub show_notes: &'static str,
    pub show_files: &'static str,
    pub width: &'static str,
    pub height: &'static str,
    pub auto_hide: &'static str,
    pub summon: &'static str,
    pub hotkey: &'static str,
    pub hotkey_press: &'static str,
    pub hotkey_listening: &'static str,
    pub hotkey_off: &'static str,
    pub hotkey_invalid: &'static str,
    pub hotkey_unavailable: &'static str,
    pub edge: &'static str,
    pub edge_dwell: &'static str,
    pub edge_unavailable: &'static str,
    pub storage: &'static str,
    pub store_copy: &'static str,
    pub store_link: &'static str,
    pub store_hint: &'static str,
    pub browser: &'static str,
    pub api_enabled: &'static str,
    pub api_port: &'static str,
    pub api_token: &'static str,
    pub copy: &'static str,
    pub regenerate: &'static str,
    pub regenerate_hint: &'static str,
    pub restart_note: &'static str,
    pub about: &'static str,
    pub open_data_folder: &'static str,
    pub open_config_file: &'static str,
    pub version: &'static str,
    // tray
    pub tray_toggle: &'static str,
    pub tray_settings: &'static str,
    pub tray_quit: &'static str,
    pub tray_tooltip: &'static str,
}

impl Strings {
    pub fn files_added(&self, lang: Lang, n: usize) -> String {
        match lang {
            Lang::En if n == 1 => "Added 1 file".to_string(),
            Lang::En => format!("Added {n} files"),
            Lang::Ja => format!("{n} 件のファイルを追加しました"),
        }
    }

    pub fn add_failed(&self, lang: Lang, name: &str, why: &str) -> String {
        match lang {
            Lang::En => format!("Couldn't add {name}: {why}"),
            Lang::Ja => format!("{name} を追加できませんでした: {why}"),
        }
    }

    pub fn count_notes(&self, lang: Lang, n: usize) -> String {
        match lang {
            Lang::En => format!("{} ({n})", self.notes),
            Lang::Ja => format!("{} ({n})", self.notes),
        }
    }

    pub fn count_files(&self, lang: Lang, n: usize) -> String {
        match lang {
            Lang::En => format!("{} ({n})", self.files),
            Lang::Ja => format!("{} ({n})", self.files),
        }
    }

    pub fn conflict_port(&self, lang: Lang, wanted: u16, got: u16) -> String {
        match lang {
            Lang::En => format!("Port {wanted} is taken; the browser API is on {got} instead"),
            Lang::Ja => {
                format!("ポート {wanted} は使用中のため、ブラウザ連携は {got} で待ち受けています")
            }
        }
    }
}

pub static EN: Strings = Strings {
    search_hint: "Search",
    keep_open: "Keep open",
    settings: "Settings",
    hide: "Hide",
    notes: "Notes",
    new_note: "New note",
    untitled: "Untitled",
    notes_empty: "No notes yet. Start typing, or press New note.",
    note_hint: "Write something…",
    pin: "Pin",
    unpin: "Unpin",
    copy_text: "Copy text",
    delete: "Delete",
    no_matches: "No matches",
    files: "Files",
    add_files: "Add files…",
    files_empty: "Drop files here",
    drop_to_add: "Drop to add to the shelf",
    open: "Open",
    show_in_folder: "Show in folder",
    copy_file: "Copy file",
    copy_path: "Copy path",
    save_copy: "Save a copy…",
    rename: "Rename",
    remove: "Remove from shelf",
    missing: "missing",
    linked: "link",
    folder: "folder",
    from: "From",
    added: "Added",
    undo: "Undo",
    cancel: "Cancel",
    copied: "Copied",
    note_deleted: "Note deleted",
    file_removed: "Removed from the shelf",
    back: "Back",
    general: "General",
    language: "Language",
    language_auto: "Automatic",
    theme: "Theme",
    theme_system: "Follow system",
    theme_light: "Light",
    theme_dark: "Dark",
    launch_at_login: "Start at login",
    panel: "Panel",
    show_notes: "Show notes",
    show_files: "Show files",
    width: "Width",
    height: "Height",
    auto_hide: "Hide when the pointer leaves",
    summon: "Summon",
    hotkey: "Shortcut",
    hotkey_press: "Click, then press the keys",
    hotkey_listening: "Press the shortcut…",
    hotkey_off: "Off",
    hotkey_invalid: "That shortcut can't be used",
    hotkey_unavailable: "Global shortcuts are not available here. Bind `tsumiba toggle` in your desktop settings instead.",
    edge: "Reveal at the top edge",
    edge_dwell: "Delay (ms)",
    edge_unavailable: "Not available on Wayland. Use the shortcut or `tsumiba toggle`.",
    storage: "Files",
    store_copy: "Keep a copy",
    store_link: "Link to the original",
    store_hint: "Folders are always linked.",
    browser: "Browser extension",
    api_enabled: "Accept the browser extension and command line",
    api_port: "Port",
    api_token: "Token",
    copy: "Copy",
    regenerate: "Regenerate",
    regenerate_hint: "Regenerating signs out every paired browser.",
    restart_note: "Takes effect after restarting tsumiba.",
    about: "About",
    open_data_folder: "Open data folder",
    open_config_file: "Open config file",
    version: "Version",
    tray_toggle: "Show / hide",
    tray_settings: "Settings…",
    tray_quit: "Quit",
    tray_tooltip: "tsumiba — memos and files",
};

pub static JA: Strings = Strings {
    search_hint: "検索",
    keep_open: "開いたままにする",
    settings: "設定",
    hide: "隠す",
    notes: "メモ",
    new_note: "新しいメモ",
    untitled: "無題",
    notes_empty: "まだメモがありません。書き始めるか「新しいメモ」を押してください。",
    note_hint: "ここに書く…",
    pin: "ピン留め",
    unpin: "ピン留めを外す",
    copy_text: "テキストをコピー",
    delete: "削除",
    no_matches: "見つかりません",
    files: "ファイル",
    add_files: "ファイルを追加…",
    files_empty: "ここにファイルをドロップ",
    drop_to_add: "ドロップして積む",
    open: "開く",
    show_in_folder: "フォルダーで表示",
    copy_file: "ファイルをコピー",
    copy_path: "パスをコピー",
    save_copy: "コピーを保存…",
    rename: "名前を変更",
    remove: "積み場から外す",
    missing: "見つかりません",
    linked: "リンク",
    folder: "フォルダー",
    from: "取得元",
    added: "追加",
    undo: "元に戻す",
    cancel: "キャンセル",
    copied: "コピーしました",
    note_deleted: "メモを削除しました",
    file_removed: "積み場から外しました",
    back: "戻る",
    general: "一般",
    language: "言語",
    language_auto: "自動",
    theme: "テーマ",
    theme_system: "システムに合わせる",
    theme_light: "ライト",
    theme_dark: "ダーク",
    launch_at_login: "ログイン時に起動",
    panel: "パネル",
    show_notes: "メモを表示",
    show_files: "ファイルを表示",
    width: "幅",
    height: "高さ",
    auto_hide: "ポインターが外れたら隠す",
    summon: "呼び出し",
    hotkey: "ショートカット",
    hotkey_press: "クリックしてキーを押す",
    hotkey_listening: "ショートカットを押してください…",
    hotkey_off: "オフ",
    hotkey_invalid: "このショートカットは使えません",
    hotkey_unavailable: "この環境ではグローバルショートカットを使えません。デスクトップの設定で `tsumiba toggle` を割り当ててください。",
    edge: "画面上端で表示",
    edge_dwell: "待ち時間 (ms)",
    edge_unavailable: "Wayland では使えません。ショートカットか `tsumiba toggle` を使ってください。",
    storage: "ファイル",
    store_copy: "コピーを保存する",
    store_link: "元の場所を参照する",
    store_hint: "フォルダーは常に参照になります。",
    browser: "ブラウザ拡張",
    api_enabled: "ブラウザ拡張とコマンドラインを受け付ける",
    api_port: "ポート",
    api_token: "トークン",
    copy: "コピー",
    regenerate: "再生成",
    regenerate_hint: "再生成すると、ペアリング済みのブラウザはすべてサインアウトされます。",
    restart_note: "tsumiba を再起動すると反映されます。",
    about: "このアプリについて",
    open_data_folder: "データフォルダーを開く",
    open_config_file: "設定ファイルを開く",
    version: "バージョン",
    tray_toggle: "表示 / 非表示",
    tray_settings: "設定…",
    tray_quit: "終了",
    tray_tooltip: "tsumiba — メモとファイルの積み場",
};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn locale_detection() {
        assert_eq!(from_locale(Some("ja-JP")), Lang::Ja);
        assert_eq!(from_locale(Some("ja")), Lang::Ja);
        assert_eq!(from_locale(Some("JA_JP.UTF-8")), Lang::Ja);
        assert_eq!(from_locale(Some("en-US")), Lang::En);
        assert_eq!(from_locale(Some("de-DE")), Lang::En);
        assert_eq!(from_locale(None), Lang::En);
    }

    #[test]
    fn explicit_setting_beats_the_locale() {
        assert_eq!(resolve(Language::Ja), Lang::Ja);
        assert_eq!(resolve(Language::En), Lang::En);
    }

    #[test]
    fn no_string_is_empty_in_either_language() {
        // A struct of &'static str cannot be iterated, so spot-check the ones a
        // careless copy-paste would blank out, and the formatters.
        for lang in [Lang::En, Lang::Ja] {
            let s = strings(lang);
            for text in [
                s.notes,
                s.files,
                s.settings,
                s.new_note,
                s.add_files,
                s.undo,
                s.tray_quit,
            ] {
                assert!(!text.is_empty());
            }
            assert!(s.files_added(lang, 2).contains('2'));
            assert!(s.add_failed(lang, "a.txt", "nope").contains("a.txt"));
        }
    }

    #[test]
    fn japanese_strings_are_actually_translated() {
        assert_ne!(EN.notes, JA.notes);
        assert_ne!(EN.files, JA.files);
        assert_ne!(EN.tray_quit, JA.tray_quit);
    }
}
