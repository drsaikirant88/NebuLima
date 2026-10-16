//! Contract test for the Settings & diagnostics IPC surface. The `#[tauri::command]`
//! wrappers in `commands/settings.rs` are thin pass-throughs over `state::NebuLimaSettings`;
//! this test exercises that persistence/patch logic directly, including the
//! absent-vs-null distinction `update_settings`'s `Partial<NebuLimaSettings>` input relies on.
use nebulima_lib::state::{NebuLimaSettings, SettingsPatch, ThemeMode};

#[test]
fn default_settings_match_contract_defaults() {
    let settings = NebuLimaSettings::default();
    assert_eq!(settings.theme_mode, ThemeMode::Dark);
    assert_eq!(settings.last_active_profile_name, None);
}

#[test]
fn save_then_load_round_trips() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("settings.json");

    let settings = NebuLimaSettings {
        theme_mode: ThemeMode::Light,
        last_active_profile_name: Some("work".to_string()),
    };
    settings.save(&path).unwrap();

    let loaded = NebuLimaSettings::load(&path);
    assert_eq!(loaded.theme_mode, ThemeMode::Light);
    assert_eq!(loaded.last_active_profile_name, Some("work".to_string()));
}

#[test]
fn load_falls_back_to_defaults_when_file_is_missing() {
    let tmp = tempfile::tempdir().unwrap();
    let path = tmp.path().join("does-not-exist.json");
    let loaded = NebuLimaSettings::load(&path);
    assert_eq!(loaded.theme_mode, ThemeMode::Dark);
}

#[test]
fn absent_patch_field_leaves_existing_value_untouched() {
    let mut settings = NebuLimaSettings {
        theme_mode: ThemeMode::Light,
        last_active_profile_name: Some("work".to_string()),
    };
    // `{}` — field entirely absent from the JSON body.
    let patch: SettingsPatch = serde_json::from_str("{}").unwrap();
    settings.apply_patch(patch);
    assert_eq!(settings.theme_mode, ThemeMode::Light);
    assert_eq!(settings.last_active_profile_name, Some("work".to_string()));
}

#[test]
fn null_patch_field_explicitly_clears_the_value() {
    let mut settings = NebuLimaSettings {
        theme_mode: ThemeMode::Light,
        last_active_profile_name: Some("work".to_string()),
    };
    // `{ "lastActiveProfileName": null }` — present but explicitly null.
    let patch: SettingsPatch = serde_json::from_str(r#"{"lastActiveProfileName": null}"#).unwrap();
    settings.apply_patch(patch);
    assert_eq!(settings.last_active_profile_name, None);
}

#[test]
fn present_patch_field_overwrites_the_value() {
    let mut settings = NebuLimaSettings::default();
    let patch: SettingsPatch = serde_json::from_str(r#"{"themeMode": "light"}"#).unwrap();
    settings.apply_patch(patch);
    assert_eq!(settings.theme_mode, ThemeMode::Light);
}

#[test]
fn settings_serialize_to_camel_case_json() {
    let settings = NebuLimaSettings {
        theme_mode: ThemeMode::Dark,
        last_active_profile_name: Some("default".to_string()),
    };
    let json = serde_json::to_value(&settings).unwrap();
    assert_eq!(json["themeMode"], "dark");
    assert_eq!(json["lastActiveProfileName"], "default");
}
