use crate::app::available_fonts;
use crate::commands::AitextApp;
use crate::config::{save_config, ApiProfile, ConfigError, StatusItem, ThemeName};
use crate::i18n::{
    known_models_count, text, FailureReason, Locale, TextKey, UiLanguage, UiMessage,
};
use crate::secrets::{remove_profile_api_key, store_profile_api_key};
use crate::theme::ShellColors;
use aitext_ai::{
    fetch_models, test_connection, AdapterKind, CompletionError, ProfileRequestConfig, ProviderKind,
};
use egui::Ui;
use std::sync::mpsc::TryRecvError;

const SETTINGS_FOOTER_HEIGHT: f32 = 46.0;

fn settings_content_height(available_height: f32) -> f32 {
    (available_height - SETTINGS_FOOTER_HEIGHT).max(180.0)
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum SettingsSection {
    #[default]
    Profiles,
    Appearance,
    StatusBar,
}

impl SettingsSection {
    pub fn all() -> [Self; 3] {
        [Self::Profiles, Self::Appearance, Self::StatusBar]
    }

    pub fn label(self, locale: Locale) -> &'static str {
        text(
            locale,
            match self {
                Self::Profiles => TextKey::SettingsProfiles,
                Self::Appearance => TextKey::SettingsAppearance,
                Self::StatusBar => TextKey::SettingsStatusBar,
            },
        )
    }
}

fn provider_label(locale: Locale, provider: ProviderKind) -> &'static str {
    let key = match provider {
        ProviderKind::DeepSeek | ProviderKind::DeepSeekFim => TextKey::ProviderDeepSeek,
        ProviderKind::OpenAi => TextKey::ProviderOpenAi,
        ProviderKind::Xai => TextKey::ProviderXai,
        ProviderKind::Anthropic => TextKey::ProviderAnthropic,
        ProviderKind::Custom => TextKey::ProviderCustom,
    };
    text(locale, key)
}

fn adapter_label(locale: Locale, adapter: AdapterKind) -> &'static str {
    let key = match adapter {
        AdapterKind::Fim => TextKey::AdapterFim,
        AdapterKind::ChatCompletions => TextKey::AdapterChatCompletions,
        AdapterKind::Responses => TextKey::AdapterResponses,
    };
    text(locale, key)
}

fn language_label(locale: Locale, language: UiLanguage) -> &'static str {
    let key = match language {
        UiLanguage::System => TextKey::LanguageSystem,
        UiLanguage::ZhCn => TextKey::LanguageZhCn,
        UiLanguage::En => TextKey::LanguageEnglish,
    };
    text(locale, key)
}

fn theme_label(locale: Locale, theme: ThemeName) -> &'static str {
    let key = match theme {
        ThemeName::White => TextKey::ThemeWhite,
        ThemeName::BlackGreen => TextKey::ThemeBlackGreen,
        ThemeName::VsCode => TextKey::ThemeVsCode,
        ThemeName::MacOs => TextKey::ThemeMacOs,
        ThemeName::Dark => TextKey::ThemeDark,
        ThemeName::Lamp => TextKey::ThemeLamp,
        ThemeName::HighContrast => TextKey::ThemeHighContrast,
        ThemeName::Custom => TextKey::ThemeCustom,
    };
    text(locale, key)
}

fn status_item_label(locale: Locale, item: StatusItem) -> &'static str {
    let key = match item {
        StatusItem::Cursor => TextKey::StatusCursor,
        StatusItem::Encoding => TextKey::StatusEncoding,
        StatusItem::Newline => TextKey::StatusNewline,
        StatusItem::Language => TextKey::StatusLanguage,
        StatusItem::Model => TextKey::StatusModel,
        StatusItem::Completion => TextKey::StatusCompletion,
        StatusItem::Message => TextKey::StatusMessage,
        StatusItem::Custom => TextKey::StatusCustom,
    };
    text(locale, key)
}

fn provider_supports_adapter_choice(provider: ProviderKind) -> bool {
    !adapter_options(provider).is_empty()
}

fn adapter_options(provider: ProviderKind) -> Vec<AdapterKind> {
    match provider {
        ProviderKind::DeepSeek => vec![
            AdapterKind::Fim,
            AdapterKind::ChatCompletions,
            AdapterKind::Responses,
        ],
        ProviderKind::OpenAi | ProviderKind::Custom => {
            vec![AdapterKind::ChatCompletions, AdapterKind::Responses]
        }
        ProviderKind::DeepSeekFim | ProviderKind::Xai | ProviderKind::Anthropic => Vec::new(),
    }
}

fn default_adapter(provider: ProviderKind) -> AdapterKind {
    provider.default_adapter()
}

fn adapter_route_hint(adapter: AdapterKind) -> &'static str {
    match adapter {
        AdapterKind::Fim => "POST /beta/completions",
        AdapterKind::ChatCompletions => "POST /chat/completions",
        AdapterKind::Responses => "POST /responses",
    }
}

fn default_base_url(provider: ProviderKind) -> &'static str {
    match provider {
        aitext_ai::ProviderKind::DeepSeek => "https://api.deepseek.com",
        aitext_ai::ProviderKind::DeepSeekFim => "https://api.deepseek.com",
        aitext_ai::ProviderKind::OpenAi => "https://api.openai.com/v1",
        aitext_ai::ProviderKind::Xai => "https://api.x.ai/v1",
        aitext_ai::ProviderKind::Anthropic => "https://api.anthropic.com",
        aitext_ai::ProviderKind::Custom => "",
    }
}

fn base_url_after_provider_change(
    old_provider: ProviderKind,
    new_provider: ProviderKind,
    current_url: &str,
) -> String {
    let current_url = current_url.trim();
    let old_default = default_base_url(old_provider);
    if current_url.is_empty() || (!old_default.is_empty() && current_url == old_default) {
        default_base_url(new_provider).to_string()
    } else {
        current_url.to_string()
    }
}

/// Result sent by a profile-scoped background operation. The worker only owns
/// a copied request configuration; it never mutates application state.
pub(crate) struct ProfileWorkerResult {
    pub(crate) profile_id: String,
    pub(crate) profile_revision: u64,
    pub(crate) operation: ProfileWorkerOperation,
    pub(crate) result: ProfileWorkerPayload,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProfileWorkerOperation {
    FetchModels,
    TestConnection,
}

pub(crate) enum ProfileWorkerPayload {
    Models(Result<Vec<String>, CompletionError>),
    Connection(Result<(), CompletionError>),
}

impl AitextApp {
    fn active_profile_worker_request_config(
        &self,
        require_model: bool,
    ) -> Option<ProfileRequestConfig> {
        let profile = self.config.active_profile()?;
        let api_key = if self.pending_api_key.trim().is_empty() {
            self.api_key.clone()?
        } else {
            self.pending_api_key.trim().to_string()
        };

        if profile.base_url.trim().is_empty()
            || api_key.is_empty()
            || (require_model && profile.selected_model.trim().is_empty())
        {
            return None;
        }

        Some(ProfileRequestConfig {
            provider: profile.provider,
            adapter: profile.adapter,
            base_url: profile.base_url.trim().to_string(),
            api_key,
            model: profile.selected_model.trim().to_string(),
            timeout_ms: profile.timeout_ms,
            allow_http: profile.allow_http,
        })
    }

    /// Starts a profile-scoped model-list request without involving the editor
    /// completion pipeline. The worker owns only an immutable request snapshot.
    pub(crate) fn fetch_active_profile_models(&mut self) {
        let Some(request_config) = self.active_profile_worker_request_config(false) else {
            self.status = Some(UiMessage::FetchModelsNeedsUrlKey);
            return;
        };
        let Some(profile) = self.config.active_profile() else {
            self.status = Some(UiMessage::FetchModelsNeedsUrlKey);
            return;
        };

        let profile_id = profile.id.clone();
        let profile_name = profile.name.clone();
        let profile_revision = self.profile_revision;
        let (sender, receiver) = std::sync::mpsc::channel();
        self.profile_worker_inboxes.push(receiver);
        self.status = Some(UiMessage::FetchingModels {
            profile: profile_name,
        });

        std::thread::spawn(move || {
            let _ = sender.send(ProfileWorkerResult {
                profile_id,
                profile_revision,
                operation: ProfileWorkerOperation::FetchModels,
                result: ProfileWorkerPayload::Models(fetch_models(&request_config)),
            });
        });
    }

    /// Starts a profile-scoped connection check with a fixed, minimal request.
    /// It never reads editor text or goes through the ghost-text pipeline.
    pub(crate) fn test_active_profile_connection(&mut self) {
        let Some(request_config) = self.active_profile_worker_request_config(true) else {
            self.status = Some(UiMessage::ConnectionNeedsUrlModelKey);
            return;
        };
        let Some(profile) = self.config.active_profile() else {
            self.status = Some(UiMessage::ConnectionNeedsUrlModelKey);
            return;
        };

        let profile_id = profile.id.clone();
        let profile_name = profile.name.clone();
        let profile_revision = self.profile_revision;
        let (sender, receiver) = std::sync::mpsc::channel();
        self.profile_worker_inboxes.push(receiver);
        self.status = Some(UiMessage::TestingConnection {
            profile: profile_name,
        });

        std::thread::spawn(move || {
            let _ = sender.send(ProfileWorkerResult {
                profile_id,
                profile_revision,
                operation: ProfileWorkerOperation::TestConnection,
                result: ProfileWorkerPayload::Connection(test_connection(&request_config)),
            });
        });
    }

    pub fn save_settings(&mut self) -> Result<(), ConfigError> {
        if let Some(profile) = self.config.active_profile_mut() {
            let selected_model = profile.selected_model.clone();
            profile.remember_model(&selected_model);
        }
        self.config = self.config.clone().clamped();
        let active_profile_id = self
            .config
            .active_profile()
            .map(|profile| profile.id.clone());
        save_config(&self.config)?;
        if let Some(profile_id) = active_profile_id {
            if !self.pending_api_key.trim().is_empty() {
                store_profile_api_key(&profile_id, &self.pending_api_key)
                    .map_err(|e| ConfigError::Io(format!("{e:?}")))?;
                self.pending_api_key_clear = false;
            } else if self.pending_api_key_clear {
                remove_profile_api_key(&profile_id)
                    .map_err(|e| ConfigError::Io(format!("{e:?}")))?;
                self.pending_api_key_clear = false;
            }
        }
        for profile_id in std::mem::take(&mut self.pending_profile_secret_deletions) {
            remove_profile_api_key(&profile_id).map_err(|e| ConfigError::Io(format!("{e:?}")))?;
        }
        self.profile_changed();
        Ok(())
    }

    /// Applies only results produced for the currently active, unmodified
    /// profile. Old workers are deliberately ignored rather than allowed to
    /// overwrite a user's newer URL, model, or provider selection.
    pub(crate) fn apply_profile_worker_result(&mut self, worker: ProfileWorkerResult) {
        let still_current = self.config.active_profile_id.as_deref()
            == Some(worker.profile_id.as_str())
            && worker.profile_revision == self.profile_revision;
        if !still_current {
            return;
        }
        let profile_name = self
            .config
            .active_profile()
            .map(|profile| profile.name.clone())
            .unwrap_or_else(|| "API profile".into());

        match (worker.operation, worker.result) {
            (ProfileWorkerOperation::FetchModels, ProfileWorkerPayload::Models(Ok(models))) => {
                let Some(profile) = self.config.active_profile_mut() else {
                    return;
                };
                let selected_model = profile.selected_model.clone();
                profile.known_models = models;
                if selected_model.trim().is_empty() {
                    if let Some(first_model) = profile.known_models.first().cloned() {
                        profile.selected_model = first_model;
                    }
                } else {
                    profile.selected_model = selected_model;
                }
                *profile = profile.clone().clamped();
                self.status = Some(UiMessage::FetchedModels {
                    count: profile.known_models.len(),
                    profile: profile.name.clone(),
                });
            }
            (ProfileWorkerOperation::FetchModels, ProfileWorkerPayload::Models(Err(error))) => {
                self.status = Some(UiMessage::FetchModelsFailed {
                    profile: profile_name,
                    reason: FailureReason::from_completion_error(&error),
                });
            }
            (ProfileWorkerOperation::TestConnection, ProfileWorkerPayload::Connection(Ok(()))) => {
                let profile_name = self
                    .config
                    .active_profile()
                    .map(|profile| profile.name.as_str())
                    .unwrap_or("API profile");
                self.status = Some(UiMessage::ConnectionVerified {
                    profile: profile_name.to_string(),
                });
            }
            (
                ProfileWorkerOperation::TestConnection,
                ProfileWorkerPayload::Connection(Err(error)),
            ) => {
                self.status = Some(UiMessage::ConnectionFailed {
                    profile: profile_name,
                    reason: FailureReason::from_completion_error(&error),
                });
            }
            _ => {}
        }
    }

    /// Drains all profile-scoped worker queues without blocking the UI thread.
    /// Disconnected workers are removed; current results are validated again by
    /// `apply_profile_worker_result` before they can change profile state.
    pub(crate) fn poll_profile_workers(&mut self) {
        let mut live_inboxes = Vec::new();
        for inbox in std::mem::take(&mut self.profile_worker_inboxes) {
            let mut connected = true;
            loop {
                match inbox.try_recv() {
                    Ok(worker) => self.apply_profile_worker_result(worker),
                    Err(TryRecvError::Empty) => break,
                    Err(TryRecvError::Disconnected) => {
                        connected = false;
                        break;
                    }
                }
            }
            if connected {
                live_inboxes.push(inbox);
            }
        }
        self.profile_worker_inboxes = live_inboxes;
    }
}
fn color_edit(ui: &mut Ui, label: &str, rgb: &mut [u8; 3]) -> bool {
    let mut changed = false;
    ui.horizontal(|ui| {
        ui.label(label);
        let mut color = egui::Color32::from_rgb(rgb[0], rgb[1], rgb[2]);
        if ui.color_edit_button_srgba(&mut color).changed() {
            *rgb = [color.r(), color.g(), color.b()];
            changed = true;
        }
    });
    changed
}

fn section_heading(ui: &mut Ui, theme: &ShellColors, title: &str, detail: &str) {
    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(title).strong().color(theme.focus));
        ui.add_space(8.0);
        ui.label(egui::RichText::new(detail).small().color(theme.muted_text));
    });
    ui.add_space(6.0);
}

fn settings_slider(ui: &mut Ui, slider: egui::Slider<'_>, theme: &ShellColors) -> egui::Response {
    ui.scope(|ui| {
        ui.visuals_mut().widgets.inactive.bg_fill = theme.rule;
        ui.visuals_mut().selection.bg_fill = theme.focus;
        ui.add(slider.trailing_fill(true))
    })
    .inner
}

fn provider_options() -> [ProviderKind; 5] {
    [
        ProviderKind::DeepSeek,
        ProviderKind::OpenAi,
        ProviderKind::Xai,
        ProviderKind::Anthropic,
        ProviderKind::Custom,
    ]
}

fn provider_hint(locale: Locale, provider: ProviderKind) -> &'static str {
    let key = match provider {
        ProviderKind::DeepSeek | ProviderKind::DeepSeekFim => TextKey::ProviderDeepSeekHint,
        ProviderKind::OpenAi => TextKey::ProviderOpenAiHint,
        ProviderKind::Xai => TextKey::ProviderXaiHint,
        ProviderKind::Anthropic => TextKey::ProviderAnthropicHint,
        ProviderKind::Custom => TextKey::ProviderCustomHint,
    };
    text(locale, key)
}

fn profile_rail_item(
    ui: &mut Ui,
    theme: &ShellColors,
    locale: Locale,
    profile: &ApiProfile,
    selected: bool,
) -> egui::Response {
    let name = if profile.name.trim().is_empty() {
        text(locale, TextKey::ProfileUnnamed)
    } else {
        profile.name.trim()
    };
    let text_color = if selected {
        theme.text
    } else {
        theme.muted_text
    };
    let fill = if selected { Some(theme.selected) } else { None };
    let mut button = egui::Button::new(
        egui::RichText::new(format!(
            "{name}\n{}",
            provider_label(locale, profile.provider)
        ))
        .size(13.0)
        .color(text_color),
    )
    .stroke(egui::Stroke::new(1.0_f32, theme.rule))
    .corner_radius(egui::CornerRadius::same(4));
    if let Some(fill) = fill {
        button = button.fill(fill);
    }
    let response = ui.add_sized([ui.available_width(), 48.0], button);
    if selected {
        let rule = egui::Rect::from_min_max(
            response.rect.min,
            egui::pos2(response.rect.min.x + 2.0, response.rect.max.y),
        );
        ui.painter()
            .rect_filled(rule, egui::CornerRadius::ZERO, theme.focus);
    }
    response
}

fn draw_profile_rail(ui: &mut Ui, app: &mut AitextApp, theme: &ShellColors) {
    let locale = app.locale();
    section_heading(
        ui,
        theme,
        text(locale, TextKey::ProfilesTitle),
        text(locale, TextKey::ProfilesDetail),
    );
    if ui.button(text(locale, TextKey::ProfilesAdd)).clicked() {
        let number = app.config.profiles.len() + 1;
        app.config.add_profile(ApiProfile::new(
            format!("{} {number}", text(locale, TextKey::ProfileTitle)),
            ProviderKind::Custom,
        ));
        app.profile_changed();
        app.status = Some(UiMessage::NewProfileAdded);
    }
    ui.add_space(8.0);

    if app.config.profiles.is_empty() {
        ui.label(
            egui::RichText::new(text(locale, TextKey::ProfilesEmpty))
                .small()
                .color(theme.muted_text),
        );
        return;
    }

    let active_id = app.config.active_profile_id.clone();
    let profiles: Vec<ApiProfile> = app.config.profiles.clone();
    for profile in profiles {
        let selected = active_id.as_deref() == Some(profile.id.as_str());
        if profile_rail_item(ui, theme, locale, &profile, selected).clicked() && !selected {
            app.activate_profile(&profile.id);
        }
        ui.add_space(5.0);
    }
}

fn draw_profile_detail(ui: &mut Ui, app: &mut AitextApp, theme: &ShellColors) {
    let locale = app.locale();
    let Some(active) = app.config.active_profile() else {
        ui.heading(egui::RichText::new(text(locale, TextKey::ProfilesNoActive)).color(theme.text));
        ui.label(
            egui::RichText::new(text(locale, TextKey::ProfilesNoActiveDetail))
                .color(theme.muted_text),
        );
        return;
    };

    let profile_id = active.id.clone();
    let mut profile_changed = false;
    let mut selected_provider = active.provider;
    let old_provider = active.provider;
    let selected_model = active.selected_model.clone();
    let models = active.known_models.clone();
    let mut fetch_models_clicked = false;

    ui.horizontal(|ui| {
        ui.heading(egui::RichText::new(text(locale, TextKey::ProfileTitle)).color(theme.text));
        ui.add_space(8.0);
        ui.label(egui::RichText::new(provider_label(locale, old_provider)).color(theme.focus));
    });
    ui.label(
        egui::RichText::new(provider_hint(locale, old_provider))
            .small()
            .color(theme.muted_text),
    );
    ui.add_space(10.0);

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(text(locale, TextKey::ProfileName)).color(theme.muted_text));
        if let Some(profile) = app.config.active_profile_mut() {
            profile_changed |= ui
                .add_sized(
                    [ui.available_width(), 26.0],
                    egui::TextEdit::singleline(&mut profile.name),
                )
                .changed();
        }
    });

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(text(locale, TextKey::ProfileProvider)).color(theme.muted_text),
        );
        egui::ComboBox::from_id_salt(("settings-provider", profile_id.clone()))
            .selected_text(provider_label(locale, selected_provider))
            .show_ui(ui, |ui| {
                for provider in provider_options() {
                    ui.selectable_value(
                        &mut selected_provider,
                        provider,
                        provider_label(locale, provider),
                    );
                }
            });
    });
    if selected_provider != old_provider {
        let current_url = app
            .config
            .active_profile()
            .map(|profile| profile.base_url.clone())
            .unwrap_or_default();
        let next_url =
            base_url_after_provider_change(old_provider, selected_provider, &current_url);
        if let Some(profile) = app.config.active_profile_mut() {
            profile.provider = selected_provider;
            profile.adapter = default_adapter(selected_provider);
            profile.base_url = next_url;
        }
        profile_changed = true;
    }

    if provider_supports_adapter_choice(selected_provider) {
        let selected_adapter = app
            .config
            .active_profile()
            .map(|profile| profile.adapter)
            .unwrap_or_else(|| default_adapter(selected_provider));
        let mut next_adapter = selected_adapter;
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(text(locale, TextKey::ProfileAdapter)).color(theme.muted_text),
            );
            egui::ComboBox::from_id_salt(("settings-adapter", profile_id.clone()))
                .selected_text(adapter_label(locale, next_adapter))
                .show_ui(ui, |ui| {
                    for adapter in adapter_options(selected_provider) {
                        ui.selectable_value(
                            &mut next_adapter,
                            adapter,
                            adapter_label(locale, adapter),
                        );
                    }
                });
            ui.label(
                egui::RichText::new(adapter_route_hint(next_adapter))
                    .small()
                    .color(theme.muted_text),
            );
        });
        if next_adapter != selected_adapter {
            if let Some(profile) = app.config.active_profile_mut() {
                profile.adapter = next_adapter;
            }
            profile_changed = true;
        }
    }

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(text(locale, TextKey::ProfileBaseUrl)).color(theme.muted_text),
        );
        if let Some(profile) = app.config.active_profile_mut() {
            profile_changed |= ui
                .add_sized(
                    [ui.available_width(), 26.0],
                    egui::TextEdit::singleline(&mut profile.base_url).hint_text("https://…"),
                )
                .changed();
        }
    });

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(text(locale, TextKey::ProfileApiKey)).color(theme.muted_text));
        let response = ui.add_sized(
            [ui.available_width() - 86.0, 26.0],
            egui::TextEdit::singleline(&mut app.pending_api_key)
                .password(true)
                .hint_text(if app.api_key.is_some() {
                    text(locale, TextKey::ProfileApiKeySavedHint)
                } else {
                    text(locale, TextKey::ProfileApiKeyPasteHint)
                }),
        );
        if response.changed() {
            note_api_key_draft_changed(app);
        }
        if ui
            .small_button(text(locale, TextKey::CommonClear))
            .clicked()
        {
            app.clear_active_profile_api_key();
            app.status = Some(UiMessage::ApiKeyCleared);
        }
    });

    ui.horizontal(|ui| {
        ui.label(egui::RichText::new(text(locale, TextKey::ProfileModel)).color(theme.muted_text));
        let mut next_model = selected_model.clone();
        egui::ComboBox::from_id_salt(("settings-model", profile_id.clone()))
            .width((ui.available_width() - 112.0).max(150.0))
            .selected_text(if selected_model.is_empty() {
                text(locale, TextKey::ProfileModelSelectHint)
            } else {
                selected_model.as_str()
            })
            .show_ui(ui, |ui| {
                for model in &models {
                    ui.selectable_value(&mut next_model, model.clone(), model);
                }
            });
        if ui
            .button(text(locale, TextKey::ProfileFetchModels))
            .clicked()
        {
            app.profile_delete_pending = None;
            if next_model != selected_model {
                if let Some(profile) = app.config.active_profile_mut() {
                    profile.selected_model = next_model.clone();
                }
                profile_changed = true;
            }
            fetch_models_clicked = true;
        } else if next_model != selected_model {
            if let Some(profile) = app.config.active_profile_mut() {
                profile.selected_model = next_model;
            }
            profile_changed = true;
        }
    });

    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(text(locale, TextKey::ProfileAddModel)).color(theme.muted_text),
        );
        let response = ui.add_sized(
            [ui.available_width() - 54.0, 26.0],
            egui::TextEdit::singleline(&mut app.pending_model)
                .hint_text(text(locale, TextKey::ProfileModelIdHint)),
        );
        let add_clicked = ui.button(text(locale, TextKey::CommonAdd)).clicked()
            || (response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)));
        if add_clicked {
            let model = app.pending_model.clone();
            if let Some(profile) = app.config.active_profile_mut() {
                profile.remember_model(&model);
            }
            app.pending_model.clear();
            profile_changed = true;
        }
    });

    let mut test_clicked = false;
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(text(locale, TextKey::ProfileConnection)).color(theme.muted_text),
        );
        test_clicked = ui
            .button(text(locale, TextKey::ProfileTestConnection))
            .clicked();
        if let Some(profile) = app.config.active_profile_mut() {
            profile_changed |= settings_slider(
                ui,
                egui::Slider::new(&mut profile.timeout_ms, 1000..=30000)
                    .text(text(locale, TextKey::ProfileTimeout)),
                theme,
            )
            .changed();
        }
    });
    if let Some(profile) = app.config.active_profile_mut() {
        profile_changed |= ui
            .checkbox(
                &mut profile.allow_http,
                text(locale, TextKey::ProfileAllowHttp),
            )
            .changed();
    }

    ui.add_space(4.0);
    if let Some(profile) = app.config.active_profile() {
        ui.label(
            egui::RichText::new(known_models_count(locale, profile.known_models.len()))
                .small()
                .color(theme.muted_text),
        );
    }

    ui.add_space(10.0);
    ui.separator();
    ui.horizontal(|ui| {
        if app.profile_delete_pending.as_deref() == Some(profile_id.as_str()) {
            ui.label(
                egui::RichText::new(text(locale, TextKey::ProfileConfirmRemove))
                    .color(theme.focus)
                    .strong(),
            );
            if ui.button(text(locale, TextKey::CommonRemove)).clicked() {
                app.remove_profile(&profile_id);
                app.status = Some(UiMessage::ProfileRemoved);
            }
            if ui.button(text(locale, TextKey::CommonKeep)).clicked() {
                app.profile_delete_pending = None;
            }
        } else if ui.button(text(locale, TextKey::ProfileRemove)).clicked() {
            app.profile_delete_pending = Some(profile_id.clone());
        }
    });

    apply_profile_detail_actions(app, profile_changed, fetch_models_clicked, test_clicked);
}

fn note_api_key_draft_changed(app: &mut AitextApp) {
    app.pending_api_key_clear = false;
    app.profile_edited();
    app.status = Some(UiMessage::DraftApiKeyChanged);
}

/// Applies UI actions after all form values are committed.  A profile edit
/// invalidates old workers and advances the revision before a new model fetch
/// or connection check snapshots the active profile.
fn apply_profile_detail_actions(
    app: &mut AitextApp,
    profile_changed: bool,
    fetch_models_clicked: bool,
    test_connection_clicked: bool,
) {
    if profile_changed {
        app.profile_edited();
    }
    if fetch_models_clicked {
        app.fetch_active_profile_models();
    }
    if test_connection_clicked {
        app.test_active_profile_connection();
    }
}

fn draw_appearance(ui: &mut Ui, app: &mut AitextApp, theme: &ShellColors) {
    let locale = app.locale();
    section_heading(
        ui,
        theme,
        text(locale, TextKey::AppearanceTitle),
        text(locale, TextKey::AppearanceDetail),
    );
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(text(locale, TextKey::AppearanceLanguage)).color(theme.muted_text),
        );
        let mut selected = app.config.ui_language;
        egui::ComboBox::from_id_salt("settings-language")
            .selected_text(language_label(locale, selected))
            .show_ui(ui, |ui| {
                for language in [UiLanguage::System, UiLanguage::ZhCn, UiLanguage::En] {
                    ui.selectable_value(&mut selected, language, language_label(locale, language));
                }
            });
        if selected != app.config.ui_language {
            app.set_ui_language(selected);
        }
    });
    let locale = app.locale();
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(text(locale, TextKey::AppearanceTheme)).color(theme.muted_text),
        );
        egui::ComboBox::from_id_salt("settings-theme")
            .selected_text(theme_label(locale, app.config.theme))
            .show_ui(ui, |ui| {
                for item in ThemeName::all() {
                    ui.selectable_value(&mut app.config.theme, item, theme_label(locale, item));
                }
            });
    });
    if app.config.theme == ThemeName::Custom {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(text(locale, TextKey::AppearanceCustomColors))
                    .color(theme.muted_text),
            );
            ui.vertical(|ui| {
                color_edit(
                    ui,
                    text(locale, TextKey::AppearancePaper),
                    &mut app.config.custom_theme.paper,
                );
                color_edit(
                    ui,
                    text(locale, TextKey::AppearanceText),
                    &mut app.config.custom_theme.text,
                );
                color_edit(
                    ui,
                    text(locale, TextKey::AppearanceAccent),
                    &mut app.config.custom_theme.accent,
                );
                color_edit(
                    ui,
                    text(locale, TextKey::AppearanceChrome),
                    &mut app.config.custom_theme.chrome,
                );
            });
        });
    }
    ui.horizontal(|ui| {
        ui.label(
            egui::RichText::new(text(locale, TextKey::AppearanceFont)).color(theme.muted_text),
        );
        egui::ComboBox::from_id_salt("settings-font")
            .selected_text(&app.config.font_family)
            .show_ui(ui, |ui| {
                for (name, _) in available_fonts() {
                    ui.selectable_value(&mut app.config.font_family, name.clone(), name);
                }
            });
        settings_slider(
            ui,
            egui::Slider::new(&mut app.config.font_size, 10.0..=28.0)
                .text(text(locale, TextKey::AppearancePoints)),
            theme,
        );
    });
    ui.horizontal(|ui| {
        ui.checkbox(
            &mut app.config.word_wrap,
            text(locale, TextKey::AppearanceWordWrap),
        );
        ui.checkbox(
            &mut app.config.use_tabs,
            text(locale, TextKey::AppearanceUseTabs),
        );
        settings_slider(
            ui,
            egui::Slider::new(&mut app.config.tab_width, 1..=8)
                .text(text(locale, TextKey::AppearanceTabWidth)),
            theme,
        );
    });
    ui.horizontal(|ui| {
        ui.checkbox(
            &mut app.config.ghost_enabled,
            text(locale, TextKey::AppearanceGhostText),
        );
        settings_slider(
            ui,
            egui::Slider::new(&mut app.config.debounce_ms, 30..=800)
                .text(text(locale, TextKey::AppearanceDebounce)),
            theme,
        );
    });
}

fn draw_status_bar_settings(ui: &mut Ui, app: &mut AitextApp, theme: &ShellColors) {
    let locale = app.locale();
    section_heading(
        ui,
        theme,
        text(locale, TextKey::StatusTitle),
        text(locale, TextKey::StatusDetail),
    );
    ui.columns(2, |columns| {
        for (index, item) in StatusItem::all().into_iter().enumerate() {
            let column = index % 2;
            let mut on = app.config.status_items.contains(&item);
            if columns[column]
                .checkbox(&mut on, status_item_label(locale, item))
                .changed()
            {
                if on {
                    if !app.config.status_items.contains(&item) {
                        app.config.status_items.push(item);
                    }
                } else {
                    app.config.status_items.retain(|existing| *existing != item);
                }
            }
        }
    });
    if app.config.status_items.contains(&StatusItem::Custom) {
        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(text(locale, TextKey::StatusCustom)).color(theme.muted_text),
            );
            ui.add_sized(
                [ui.available_width(), 26.0],
                egui::TextEdit::singleline(&mut app.config.status_custom)
                    .hint_text(text(locale, TextKey::StatusCustomHint)),
            );
        });
    }
}

pub fn draw_settings(ui: &mut Ui, app: &mut AitextApp, close: &mut bool) {
    let theme = crate::theme::shell_colors(app.config.theme);
    let locale = app.locale();
    let content_height = settings_content_height(ui.available_height());
    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), content_height),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.heading(egui::RichText::new(text(locale, TextKey::SettingsTitle)).color(theme.text));
            ui.label(
                egui::RichText::new(text(locale, TextKey::SettingsIntro))
                    .small()
                    .color(theme.muted_text),
            );
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                for section in SettingsSection::all() {
                    let selected = app.settings_section == section;
                    let fill = if selected {
                        theme.selected
                    } else {
                        egui::Color32::TRANSPARENT
                    };
                    let text = if selected {
                        theme.text
                    } else {
                        theme.muted_text
                    };
                    let button = egui::Button::new(
                        egui::RichText::new(section.label(locale))
                            .color(text)
                            .strong(),
                    )
                    .fill(fill)
                    .stroke(egui::Stroke::new(1.0_f32, theme.rule))
                    .corner_radius(egui::CornerRadius::same(3));
                    if ui.add_sized([126.0, 30.0], button).clicked() {
                        app.settings_section = section;
                    }
                }
            });
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);

            egui::ScrollArea::vertical()
                .id_salt(("settings-scroll", app.settings_section))
                .auto_shrink([false, false])
                .max_height(ui.available_height())
                .show(ui, |ui| {
                    ui.set_min_width(ui.available_width());
                    match app.settings_section {
                        SettingsSection::Profiles => {
                            ui.horizontal_top(|ui| {
                                let rail_width = (ui.available_width() * 0.29).clamp(190.0, 250.0);
                                ui.allocate_ui_with_layout(
                                    egui::vec2(rail_width, 0.0),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| draw_profile_rail(ui, app, &theme),
                                );
                                ui.add_space(18.0);
                                ui.separator();
                                ui.add_space(18.0);
                                ui.allocate_ui_with_layout(
                                    egui::vec2(ui.available_width(), 0.0),
                                    egui::Layout::top_down(egui::Align::Min),
                                    |ui| draw_profile_detail(ui, app, &theme),
                                );
                            });
                        }
                        SettingsSection::Appearance => draw_appearance(ui, app, &theme),
                        SettingsSection::StatusBar => draw_status_bar_settings(ui, app, &theme),
                    }
                });
        },
    );

    ui.allocate_ui_with_layout(
        egui::vec2(ui.available_width(), SETTINGS_FOOTER_HEIGHT),
        egui::Layout::top_down(egui::Align::Min),
        |ui| {
            ui.separator();
            ui.add_space(4.0);
            let locale = app.locale();
            ui.horizontal(|ui| {
                if ui.button(text(locale, TextKey::SettingsSave)).clicked() {
                    match app.save_settings() {
                        Ok(()) => app.status = Some(UiMessage::SettingsSaved),
                        Err(err) => app.status = Some(UiMessage::SaveFailed(format!("{err:?}"))),
                    }
                }
                if let Some(status) = app.status.as_ref() {
                    let status_color = if status.is_error() {
                        theme.focus
                    } else {
                        theme.muted_text
                    };
                    ui.label(
                        egui::RichText::new(status.render(app.locale()))
                            .small()
                            .color(status_color),
                    );
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    if ui.button(text(locale, TextKey::CommonClose)).clicked() {
                        *close = true;
                    }
                });
            });
        },
    );
}

#[cfg(test)]
mod tests {
    use super::{
        adapter_label, adapter_options, apply_profile_detail_actions,
        base_url_after_provider_change, default_adapter, default_base_url, draw_appearance,
        language_label, note_api_key_draft_changed, provider_hint, provider_label,
        provider_supports_adapter_choice, settings_content_height, ProfileWorkerOperation,
        ProfileWorkerPayload, ProfileWorkerResult, SettingsSection,
    };
    use crate::commands::AitextApp;
    use crate::config::ApiProfile;
    use crate::i18n::{Locale, UiLanguage};
    use crate::secrets::{
        load_api_key, load_profile_api_key, store_api_key, store_profile_api_key,
    };
    use aitext_ai::{AdapterKind, CompletionError, ProviderKind};

    #[test]
    fn settings_sections_are_localized() {
        assert_eq!(
            SettingsSection::all(),
            [
                SettingsSection::Profiles,
                SettingsSection::Appearance,
                SettingsSection::StatusBar,
            ]
        );
        assert_eq!(SettingsSection::default(), SettingsSection::Profiles);
        assert_eq!(SettingsSection::Profiles.label(Locale::En), "AI Profiles");
        assert_eq!(SettingsSection::Profiles.label(Locale::ZhCn), "AI 配置");
        assert_eq!(SettingsSection::Appearance.label(Locale::En), "Appearance");
        assert_eq!(SettingsSection::Appearance.label(Locale::ZhCn), "外观");
        assert_eq!(SettingsSection::StatusBar.label(Locale::En), "Status Bar");
        assert_eq!(SettingsSection::StatusBar.label(Locale::ZhCn), "状态栏");
    }

    #[test]
    fn language_options_are_self_identifying() {
        assert_eq!(
            language_label(Locale::En, UiLanguage::System),
            "Follow Windows / 跟随 Windows"
        );
        assert_eq!(
            language_label(Locale::ZhCn, UiLanguage::System),
            "跟随 Windows / Follow Windows"
        );
        assert_eq!(language_label(Locale::En, UiLanguage::ZhCn), "简体中文");
        assert_eq!(language_label(Locale::ZhCn, UiLanguage::En), "English");
    }

    #[test]
    fn new_app_starts_settings_on_profiles_section() {
        let app = AitextApp::new_for_test();
        assert_eq!(app.settings_section, SettingsSection::Profiles);
    }

    #[test]
    fn language_change_preserves_unsaved_profile_and_secret_drafts() {
        let mut app = AitextApp::new_for_test();
        app.config
            .add_profile(ApiProfile::new("Relay", ProviderKind::Custom));
        app.pending_api_key = "draft-key".into();
        app.pending_model = "draft-model".into();
        let revision = app.profile_revision;

        app.set_ui_language(UiLanguage::ZhCn);

        assert_eq!(app.pending_api_key, "draft-key");
        assert_eq!(app.pending_model, "draft-model");
        assert_eq!(app.profile_revision, revision);
        assert_eq!(app.config.active_profile().unwrap().name, "Relay");
    }

    #[test]
    fn settings_content_reserves_a_fixed_footer() {
        assert_eq!(settings_content_height(600.0), 554.0);
        assert_eq!(settings_content_height(220.0), 180.0);
    }

    #[test]
    fn appearance_sliders_show_a_track_and_value_fill() {
        let mut app = AitextApp::new_for_test();
        let shell = crate::theme::shell_colors(crate::config::ThemeName::Dark);
        let context = egui::Context::default();
        let input = egui::RawInput {
            screen_rect: Some(egui::Rect::from_min_size(
                egui::Pos2::ZERO,
                egui::vec2(720.0, 420.0),
            )),
            ..Default::default()
        };

        let output = context.run(input, |context| {
            egui::CentralPanel::default()
                .frame(egui::Frame::NONE.fill(shell.raised))
                .show(context, |ui| draw_appearance(ui, &mut app, &shell));
        });
        let slider_fills: Vec<_> = output
            .shapes
            .iter()
            .filter_map(|clipped| match &clipped.shape {
                egui::Shape::Rect(shape)
                    if (1.0..=200.0).contains(&shape.rect.width())
                        && shape.rect.height() <= 8.0 =>
                {
                    Some(shape.fill)
                }
                _ => None,
            })
            .collect();

        assert!(slider_fills.contains(&shell.rule));
        assert!(slider_fills.contains(&shell.focus));
    }

    #[test]
    fn provider_defaults_are_explicit_and_user_facing() {
        assert_eq!(
            default_base_url(ProviderKind::DeepSeek),
            "https://api.deepseek.com"
        );
        assert_eq!(
            default_base_url(ProviderKind::OpenAi),
            "https://api.openai.com/v1"
        );
        assert_eq!(default_base_url(ProviderKind::Xai), "https://api.x.ai/v1");
        assert_eq!(
            default_base_url(ProviderKind::Anthropic),
            "https://api.anthropic.com"
        );
        assert_eq!(
            provider_label(Locale::En, ProviderKind::Custom),
            "Custom provider"
        );
        assert_eq!(
            provider_label(Locale::ZhCn, ProviderKind::Custom),
            "自定义提供商"
        );
    }

    #[test]
    fn adapter_choice_is_exposed_for_deepseek_openai_and_custom_providers() {
        assert!(provider_supports_adapter_choice(ProviderKind::DeepSeek));
        assert!(provider_supports_adapter_choice(ProviderKind::OpenAi));
        assert!(provider_supports_adapter_choice(ProviderKind::Custom));
        assert!(!provider_supports_adapter_choice(ProviderKind::Xai));
        assert!(!provider_supports_adapter_choice(ProviderKind::Anthropic));
        assert_eq!(adapter_label(Locale::En, AdapterKind::Fim), "FIM");
        assert_eq!(
            adapter_label(Locale::ZhCn, AdapterKind::ChatCompletions),
            "Chat Completions"
        );
        assert_eq!(
            adapter_label(Locale::En, AdapterKind::Responses),
            "Responses API"
        );
        assert_eq!(
            provider_hint(Locale::En, ProviderKind::Custom),
            "Choose the API shape used by your custom endpoint"
        );
        assert_eq!(
            provider_hint(Locale::En, ProviderKind::DeepSeek),
            "Choose FIM, Chat Completions, or Responses API"
        );
    }

    #[test]
    fn deepseek_adapter_options_include_fim_chat_and_responses() {
        assert_eq!(
            adapter_options(ProviderKind::DeepSeek),
            vec![
                AdapterKind::Fim,
                AdapterKind::ChatCompletions,
                AdapterKind::Responses,
            ]
        );
        assert_eq!(
            adapter_options(ProviderKind::OpenAi),
            vec![AdapterKind::ChatCompletions, AdapterKind::Responses]
        );
        assert!(adapter_options(ProviderKind::Xai).is_empty());
        assert_eq!(default_adapter(ProviderKind::DeepSeek), AdapterKind::Fim);
        assert_eq!(
            default_adapter(ProviderKind::OpenAi),
            AdapterKind::ChatCompletions
        );
    }

    #[test]
    fn adapter_edit_invalidates_visible_ghost_and_old_workers() {
        let mut app = AitextApp::new_for_test();
        app.config
            .add_profile(ApiProfile::new("OpenAI", ProviderKind::OpenAi));
        app.force_ghost("stale ghost");
        let (_sender, receiver) = std::sync::mpsc::channel::<ProfileWorkerResult>();
        app.profile_worker_inboxes.push(receiver);
        let revision = app.profile_revision;

        app.config.active_profile_mut().unwrap().adapter = AdapterKind::Responses;
        apply_profile_detail_actions(&mut app, true, false, false);

        assert_eq!(
            app.config.active_profile().unwrap().adapter,
            AdapterKind::Responses
        );
        assert!(app.ghost_text().is_none());
        assert!(app.profile_worker_inboxes.is_empty());
        assert!(app.profile_revision > revision);
    }

    #[test]
    fn provider_switch_replaces_only_an_old_default_url() {
        assert_eq!(
            base_url_after_provider_change(
                ProviderKind::DeepSeek,
                ProviderKind::OpenAi,
                "https://api.deepseek.com",
            ),
            "https://api.openai.com/v1"
        );
        assert_eq!(
            base_url_after_provider_change(
                ProviderKind::DeepSeek,
                ProviderKind::OpenAi,
                "https://relay.example.test/v1",
            ),
            "https://relay.example.test/v1"
        );
        assert_eq!(
            base_url_after_provider_change(ProviderKind::Custom, ProviderKind::Xai, "",),
            "https://api.x.ai/v1"
        );
    }

    #[test]
    fn save_settings_persists_active_profile_without_key() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "aitext-settings-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("AITEXT_CONFIG_DIR", &dir);
        store_api_key("").unwrap();
        let mut app = AitextApp::new_for_test();
        let mut profile = ApiProfile::new("Test API", ProviderKind::Custom);
        profile.remember_model("m1");
        app.config.add_profile(profile);
        let profile_id = app.config.active_profile().unwrap().id.clone();
        app.pending_api_key = "sk-test".into();
        let revision_before_save = app.profile_revision;
        app.save_settings().unwrap();
        let raw = std::fs::read_to_string(dir.join("config.toml")).unwrap();
        assert!(raw.contains("Test API"));
        assert!(raw.contains("m1"));
        assert!(!raw.contains("sk-test"));
        assert_eq!(
            load_profile_api_key(&profile_id).unwrap().as_deref(),
            Some("sk-test")
        );
        assert_eq!(load_api_key().unwrap(), None);
        assert!(app
            .config
            .active_profile()
            .unwrap()
            .known_models
            .contains(&"m1".to_string()));
        assert!(app.profile_revision > revision_before_save);
    }

    #[test]
    fn save_settings_persists_language_without_plaintext_key() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "aitext-language-settings-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("AITEXT_CONFIG_DIR", &dir);
        store_api_key("").unwrap();

        let mut app = AitextApp::new_for_test();
        app.set_ui_language(UiLanguage::ZhCn);
        let mut profile = ApiProfile::new("Test API", ProviderKind::Custom);
        profile.remember_model("m1");
        app.config.add_profile(profile);
        let profile_id = app.config.active_profile().unwrap().id.clone();
        app.pending_api_key = "sk-language-test".into();

        app.save_settings().unwrap();

        let raw = std::fs::read_to_string(dir.join("config.toml")).unwrap();
        assert!(raw.contains("ui_language = \"zh_cn\""));
        assert!(!raw.contains("sk-language-test"));
        assert_eq!(
            load_profile_api_key(&profile_id).unwrap().as_deref(),
            Some("sk-language-test")
        );
        assert_eq!(load_api_key().unwrap(), None);
    }

    #[test]
    fn stale_model_fetch_result_does_not_overwrite_changed_profile() {
        let mut app = AitextApp::new_for_test();
        let mut profile = ApiProfile::new("OpenAI", ProviderKind::OpenAi);
        profile.base_url = "https://old.example.test/v1".into();
        profile.remember_model("manual-old");
        app.config.add_profile(profile);

        let profile_id = app.config.active_profile().unwrap().id.clone();
        let stale_revision = app.profile_revision;
        {
            let profile = app.config.active_profile_mut().unwrap();
            profile.base_url = "https://new.example.test/v1".into();
            profile.selected_model = "manual-new".into();
            profile.known_models = vec!["manual-new".into()];
        }
        app.profile_changed();

        app.apply_profile_worker_result(ProfileWorkerResult {
            profile_id,
            profile_revision: stale_revision,
            operation: ProfileWorkerOperation::FetchModels,
            result: ProfileWorkerPayload::Models(Ok(vec!["remote-model".into()])),
        });

        let profile = app.config.active_profile().unwrap();
        assert_eq!(profile.selected_model, "manual-new");
        assert_eq!(profile.known_models, vec!["manual-new"]);
    }

    #[test]
    fn connection_failure_status_is_profile_scoped_and_categorized() {
        let mut app = AitextApp::new_for_test();
        app.config
            .add_profile(ApiProfile::new("Claude", ProviderKind::Anthropic));
        let profile_id = app.config.active_profile().unwrap().id.clone();
        let revision = app.profile_revision;
        let cases = [
            (
                CompletionError::AuthFailed,
                "Connection failed for Claude: authentication failed.",
            ),
            (
                CompletionError::Timeout,
                "Connection failed for Claude: request timed out.",
            ),
            (
                CompletionError::RequestFailed("http 404: model not found".into()),
                "Connection failed for Claude: selected model is unavailable.",
            ),
            (
                CompletionError::RequestFailed("http 500: provider error".into()),
                "Connection failed for Claude: provider returned an HTTP error.",
            ),
        ];

        for (error, expected) in cases {
            app.apply_profile_worker_result(ProfileWorkerResult {
                profile_id: profile_id.clone(),
                profile_revision: revision,
                operation: ProfileWorkerOperation::TestConnection,
                result: ProfileWorkerPayload::Connection(Err(error)),
            });
            assert_eq!(app.status_text().as_deref(), Some(expected));
        }
    }

    #[test]
    fn model_fetch_failure_names_the_profile_and_category() {
        let mut app = AitextApp::new_for_test();
        app.config
            .add_profile(ApiProfile::new("Relay", ProviderKind::Custom));
        let profile_id = app.config.active_profile().unwrap().id.clone();

        app.apply_profile_worker_result(ProfileWorkerResult {
            profile_id,
            profile_revision: app.profile_revision,
            operation: ProfileWorkerOperation::FetchModels,
            result: ProfileWorkerPayload::Models(Err(CompletionError::AuthFailed)),
        });

        assert_eq!(
            app.status_text().as_deref(),
            Some("Could not fetch models for Relay: authentication failed.")
        );
    }

    #[test]
    fn current_model_fetch_keeps_manual_selected_model() {
        let mut app = AitextApp::new_for_test();
        let mut profile = ApiProfile::new("Relay", ProviderKind::Custom);
        profile.remember_model("manual-model");
        app.config.add_profile(profile);

        let profile_id = app.config.active_profile().unwrap().id.clone();
        app.apply_profile_worker_result(ProfileWorkerResult {
            profile_id,
            profile_revision: app.profile_revision,
            operation: ProfileWorkerOperation::FetchModels,
            result: ProfileWorkerPayload::Models(Ok(vec![
                "remote-one".into(),
                "remote-two".into(),
            ])),
        });

        let profile = app.config.active_profile().unwrap();
        assert_eq!(profile.selected_model, "manual-model");
        assert_eq!(
            profile.known_models,
            vec![
                "manual-model".to_string(),
                "remote-one".to_string(),
                "remote-two".to_string(),
            ]
        );
    }

    #[test]
    fn poll_profile_workers_applies_current_model_fetch_result() {
        let mut app = AitextApp::new_for_test();
        let mut profile = ApiProfile::new("Relay", ProviderKind::Custom);
        profile.remember_model("manual-model");
        app.config.add_profile(profile);

        let profile_id = app.config.active_profile().unwrap().id.clone();
        let (tx, rx) = std::sync::mpsc::channel();
        app.profile_worker_inboxes.push(rx);
        tx.send(ProfileWorkerResult {
            profile_id,
            profile_revision: app.profile_revision,
            operation: ProfileWorkerOperation::FetchModels,
            result: ProfileWorkerPayload::Models(Ok(vec!["remote-model".into()])),
        })
        .unwrap();

        app.poll_profile_workers();

        let profile = app.config.active_profile().unwrap();
        assert_eq!(profile.selected_model, "manual-model");
        assert_eq!(
            profile.known_models,
            vec!["manual-model".to_string(), "remote-model".to_string()]
        );
    }

    #[test]
    fn profile_edit_preserves_draft_key_and_invalidates_profile_workers() {
        let mut app = AitextApp::new_for_test();
        let mut profile = ApiProfile::new("Relay", ProviderKind::Custom);
        profile.remember_model("relay-model");
        app.config.add_profile(profile);
        app.pending_api_key = "draft-key".into();

        let (_tx, rx) = std::sync::mpsc::channel::<ProfileWorkerResult>();
        app.profile_worker_inboxes.push(rx);
        let revision_before_edit = app.profile_revision;

        app.profile_edited();

        assert_eq!(app.pending_api_key, "draft-key");
        assert!(app.profile_worker_inboxes.is_empty());
        assert!(app.profile_revision > revision_before_edit);
    }

    #[test]
    fn api_key_draft_change_invalidates_old_profile_work_without_clearing_draft() {
        let mut app = AitextApp::new_for_test();
        app.config
            .add_profile(ApiProfile::new("Relay", ProviderKind::Custom));
        app.pending_api_key = "new-draft-key".into();
        app.force_ghost("stale ghost");
        let (_tx, rx) = std::sync::mpsc::channel::<ProfileWorkerResult>();
        app.profile_worker_inboxes.push(rx);
        let revision_before_change = app.profile_revision;

        note_api_key_draft_changed(&mut app);

        assert_eq!(app.pending_api_key, "new-draft-key");
        assert!(app.ghost_text().is_none());
        assert!(app.profile_worker_inboxes.is_empty());
        assert!(app.profile_revision > revision_before_change);
        assert_eq!(
            app.status_text().as_deref(),
            Some("Draft API key changed; save to store it.")
        );
    }

    #[test]
    fn fetch_models_requires_url_and_key_without_starting_a_worker() {
        let mut app = AitextApp::new_for_test();
        app.config
            .add_profile(ApiProfile::new("Relay", ProviderKind::Custom));

        app.fetch_active_profile_models();

        assert_eq!(
            app.status_text().as_deref(),
            Some("Fetch models needs a URL and API key.")
        );
        assert!(app.profile_worker_inboxes.is_empty());
    }

    #[test]
    fn connection_test_requires_url_model_and_key_without_starting_a_worker() {
        let mut app = AitextApp::new_for_test();
        app.config
            .add_profile(ApiProfile::new("Relay", ProviderKind::Custom));

        app.test_active_profile_connection();

        assert_eq!(
            app.status_text().as_deref(),
            Some("Connection test needs a URL, model, and API key.")
        );
        assert!(app.profile_worker_inboxes.is_empty());
        assert_eq!(app.api_key, None);
    }

    #[test]
    fn connection_test_uses_draft_key_and_fixed_snapshot_without_document_text() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::mpsc;
        use std::time::{Duration, Instant};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (request_tx, request_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let read = stream.read(&mut request).unwrap();
            request_tx
                .send(String::from_utf8_lossy(&request[..read]).into_owned())
                .unwrap();
            let body = r#"{"choices":[{"message":{"content":"ok"}}]}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });

        let mut app = AitextApp::new_for_test();
        app.workspace.new_untitled();
        app.workspace
            .current_mut()
            .unwrap()
            .insert("PRIVATE_DOCUMENT_TEXT");
        let mut profile = ApiProfile::new("Relay", ProviderKind::Custom);
        profile.base_url = format!("http://{address}/v1");
        profile.allow_http = true;
        profile.remember_model("relay-model");
        app.config.add_profile(profile);
        app.pending_api_key = "draft-key".into();

        app.test_active_profile_connection();

        assert_eq!(
            app.status_text().as_deref(),
            Some("Testing connection for Relay…")
        );
        assert_eq!(app.profile_worker_inboxes.len(), 1);

        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline
            && app.status_text().as_deref() != Some("Connection verified for Relay.")
        {
            app.poll_profile_workers();
            std::thread::sleep(Duration::from_millis(5));
        }
        app.poll_profile_workers();

        let request = request_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(request.starts_with("POST /v1/chat/completions HTTP/1.1"));
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer draft-key"));
        assert!(request.contains("let answer = "));
        assert!(!request.contains("PRIVATE_DOCUMENT_TEXT"));
        assert_eq!(
            app.status_text().as_deref(),
            Some("Connection verified for Relay.")
        );
    }

    #[test]
    fn fetch_models_uses_the_active_profile_draft_key() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::mpsc;
        use std::time::{Duration, Instant};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (request_tx, request_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let read = stream.read(&mut request).unwrap();
            request_tx
                .send(String::from_utf8_lossy(&request[..read]).into_owned())
                .unwrap();
            let body = r#"{"data":[{"id":"relay-one"},{"id":"relay-two"}]}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });

        let mut app = AitextApp::new_for_test();
        let mut profile = ApiProfile::new("Relay", ProviderKind::Custom);
        profile.base_url = format!("http://{address}/v1");
        profile.allow_http = true;
        app.config.add_profile(profile);
        app.pending_api_key = "draft-key".into();

        app.fetch_active_profile_models();

        assert_eq!(
            app.status_text().as_deref(),
            Some("Fetching models for Relay…")
        );
        assert_eq!(app.profile_worker_inboxes.len(), 1);

        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline
            && app
                .config
                .active_profile()
                .is_some_and(|profile| profile.known_models.is_empty())
        {
            app.poll_profile_workers();
            std::thread::sleep(Duration::from_millis(5));
        }
        app.poll_profile_workers();

        let request = request_rx.recv_timeout(Duration::from_secs(1)).unwrap();
        assert!(request.starts_with("GET /v1/models HTTP/1.1"));
        assert!(request
            .to_ascii_lowercase()
            .contains("authorization: bearer draft-key"));
        let profile = app.config.active_profile().unwrap();
        assert_eq!(profile.selected_model, "relay-one");
        assert_eq!(profile.known_models, vec!["relay-one", "relay-two"]);
    }

    #[test]
    fn profile_edits_are_invalidated_before_a_model_fetch_starts() {
        use std::io::{Read, Write};
        use std::net::TcpListener;
        use std::sync::mpsc;
        use std::time::{Duration, Instant};

        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let (request_tx, request_rx) = mpsc::channel();
        std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            let mut request = [0_u8; 4096];
            let read = stream.read(&mut request).unwrap();
            request_tx
                .send(String::from_utf8_lossy(&request[..read]).into_owned())
                .unwrap();
            let body = r#"{"data":[{"id":"fresh-model"}]}"#;
            write!(
                stream,
                "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",
                body.len(),
                body
            )
            .unwrap();
        });

        let mut app = AitextApp::new_for_test();
        let mut profile = ApiProfile::new("Relay", ProviderKind::Custom);
        profile.base_url = format!("http://{address}/v1");
        profile.allow_http = true;
        app.config.add_profile(profile);
        app.pending_api_key = "test-key".into();
        let revision_before_edit = app.profile_revision;

        apply_profile_detail_actions(&mut app, true, true, false);

        assert!(app.profile_revision > revision_before_edit);
        assert_eq!(app.profile_worker_inboxes.len(), 1);
        let deadline = Instant::now() + Duration::from_secs(1);
        while Instant::now() < deadline
            && app
                .config
                .active_profile()
                .is_some_and(|profile| profile.known_models.is_empty())
        {
            app.poll_profile_workers();
            std::thread::sleep(Duration::from_millis(5));
        }
        app.poll_profile_workers();

        assert!(request_rx
            .recv_timeout(Duration::from_secs(1))
            .unwrap()
            .starts_with("GET /v1/models HTTP/1.1"));
        assert_eq!(
            app.config.active_profile().unwrap().known_models,
            vec!["fresh-model"]
        );
    }

    #[test]
    fn saving_after_clearing_a_key_removes_only_the_active_profile_secret() {
        let _guard = crate::TEST_ENV_LOCK.lock().unwrap();
        let dir = std::env::temp_dir().join(format!(
            "aitext-settings-{}",
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::env::set_var("AITEXT_CONFIG_DIR", &dir);

        let mut app = AitextApp::new_for_test();
        app.config
            .add_profile(ApiProfile::new("First", ProviderKind::OpenAi));
        app.config
            .add_profile(ApiProfile::new("Second", ProviderKind::Xai));
        let first_profile_id = app.config.profiles[0].id.clone();
        let second_profile_id = app.config.profiles[1].id.clone();
        store_profile_api_key(&first_profile_id, "first-test-key").unwrap();
        store_profile_api_key(&second_profile_id, "second-test-key").unwrap();
        assert!(app.activate_profile(&first_profile_id));
        app.reload_active_profile_key();

        app.clear_active_profile_api_key();
        app.save_settings().unwrap();

        assert_eq!(load_profile_api_key(&first_profile_id).unwrap(), None);
        assert_eq!(
            load_profile_api_key(&second_profile_id).unwrap().as_deref(),
            Some("second-test-key")
        );
    }
}
