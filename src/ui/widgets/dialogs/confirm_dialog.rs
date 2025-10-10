use crate::errors::Error;
use crate::i18n::{I18N, LangKey};
use crate::ui::Dialog;
use egui::Context;
use std::sync::{Arc, RwLock};

pub struct ConfirmDialog {
    title: String,
    message: String,
    on_confirm: Box<dyn Fn() + Send + Sync + 'static>,
    on_cancel: Option<Box<dyn Fn() + Send + Sync + 'static>>,
    show_dialog: RwLock<bool>,
}

impl ConfirmDialog {
    pub fn builder(on_confirm: Box<dyn Fn() + Send + Sync + 'static>) -> ConfirmDialogBuilder {
        ConfirmDialogBuilder::new(on_confirm)
    }

    pub fn is_open(&self) -> bool {
        match self.show_dialog.read() {
            Ok(r) => *r,
            Err(e) => {
                eprintln!("Error trying to read show dialog state: {:?}", e);
                false
            }
        }
    }
}

impl Dialog for ConfirmDialog {
    fn show(&self, ctx: &Context, i18n: Arc<I18N>) -> Result<(), Error> {
        egui::Window::new(&self.title)
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(&self.message);
                match self.show_dialog.try_write() {
                    Ok(mut show_dialog) => {
                        if ui.button(i18n.get(LangKey::Ok)).clicked() {
                            (self.on_confirm)();
                            *show_dialog = false;
                        }
                        if ui.button(i18n.get(LangKey::Cancel)).clicked() {
                            if let Some(on_cancel) = &self.on_cancel {
                                on_cancel()
                            }
                            *show_dialog = false;
                        }
                    }
                    Err(e) => {
                        eprintln!("Error trying to change show dialog state: {:?}", e);
                    }
                };
            });
        Ok(())
    }
}

pub struct ConfirmDialogBuilder {
    title: Option<String>,
    message: Option<String>,
    on_confirm: Box<dyn Fn() + Send + Sync + 'static>,
    on_cancel: Option<Box<dyn Fn() + Send + Sync + 'static>>,
}

impl ConfirmDialogBuilder {
    pub fn new(on_confirm: Box<dyn Fn() + Send + Sync + 'static>) -> Self {
        Self {
            title: None,
            message: None,
            on_confirm,
            on_cancel: None,
        }
    }

    pub fn title(mut self, title: &str) -> Self {
        self.title = Some(title.to_string());
        self
    }

    pub fn message(mut self, message: &str) -> Self {
        self.message = Some(message.to_string());
        self
    }

    pub fn on_cancel(mut self, on_cancel: Box<dyn Fn() + Send + Sync + 'static>) -> Self {
        self.on_cancel = Some(on_cancel);
        self
    }

    pub fn build(self) -> ConfirmDialog {
        ConfirmDialog {
            title: self.title.unwrap_or_else(|| "Error".to_string()),
            message: self
                .message
                .unwrap_or_else(|| "An error occurred".to_string()),
            on_confirm: self.on_confirm,
            on_cancel: self.on_cancel,
            show_dialog: RwLock::from(true),
        }
    }
}
