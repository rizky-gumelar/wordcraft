//! WordCraft Android runner (eframe on android-activity).
#![deny(clippy::unwrap_used, clippy::expect_used, clippy::panic, clippy::unimplemented, clippy::todo, clippy::unreachable)]

#[cfg(target_os = "android")]
mod android {
    use wordcraft_engine::Session;
    use wordcraft_ui_egui::{Services, WordApp};
    use winit::platform::android::activity::AndroidApp;

    pub struct AndroidAppWrapper(pub WordApp);

    impl eframe::App for AndroidAppWrapper {
        fn logic(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
            self.0.logic(ctx);
        }
        fn raw_input_hook(&mut self, _ctx: &egui::Context, raw: &mut egui::RawInput) {
            self.0.raw_input_hook(raw);
        }
        fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
            self.0.ui(ui);
        }
    }

    #[unsafe(no_mangle)]
    pub fn android_main(app: AndroidApp) {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Info)
                .with_tag("WordCraft"),
        );
        log::info!("Starting WordCraft on Android tablet (Glow/OpenGL ES)...");

        // Load custom user fonts from external files directory or tablet Documents/WordCraft/fonts
        if let Some(mut path) = app.external_data_path() {
            path.push("fonts");
            let _ = std::fs::create_dir_all(&path);
            let (loaded, _) = wordcraft_fonts::FontDb::global().load_user_fonts(&path);
            if !loaded.is_empty() {
                log::info!("Loaded {} user fonts from {:?}", loaded.len(), path);
            }
        }
        for extra in ["/sdcard/Documents/WordCraft/fonts", "/sdcard/Fonts"] {
            let p = std::path::Path::new(extra);
            if p.is_dir() {
                let (loaded, _) = wordcraft_fonts::FontDb::global().load_user_fonts(p);
                if !loaded.is_empty() {
                    log::info!("Loaded {} user fonts from {:?}", loaded.len(), p);
                }
            }
        }

        // Ensure default document save/export directory exists on Android storage
        for doc_dir in ["/sdcard/Documents/WordCraft", "/storage/emulated/0/Documents/WordCraft"] {
            let p = std::path::Path::new(doc_dir);
            let _ = std::fs::create_dir_all(p);
        }
        if let Some(mut path) = app.external_data_path() {
            path.push("Documents");
            let _ = std::fs::create_dir_all(&path);
        }

        let mut options = eframe::NativeOptions::default();
        options.android_app = Some(app);
        options.renderer = eframe::Renderer::Glow;

        if let Err(e) = eframe::run_native(
            "WordCraft",
            options,
            Box::new(|_cc| {
                let region = wordcraft_ui_egui::i18n::system_region();
                let doc = wordcraft_engine::sample::sample_document();
                let mut session = Session::new(doc);
                session.system_region = region;

                // On Android, without external file dialogs, WordCraft automatically
                // falls back to its built-in touch-friendly file browser.
                let services = Services::default();
                let mut app = WordApp::new(session, services);
                app.integrated_titlebar = false;

                // Optimize touch interactions for tablet
                for theme in [egui::Theme::Light, egui::Theme::Dark] {
                    _cc.egui_ctx.style_mut_of(theme, |style| {
                        style.interaction.interact_radius = 8.0;
                        style.spacing.button_padding = egui::vec2(6.0, 5.0);
                    });
                }

                Ok(Box::new(AndroidAppWrapper(app)))
            }),
        ) {
            log::error!("WordCraft failed to start: {e}");
        }
    }
}

#[cfg(not(target_os = "android"))]
pub fn dummy() {}
