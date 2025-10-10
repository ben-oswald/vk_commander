use crate::state::Info;

#[derive(Default, Clone)]
pub struct InfoModal {
    pub title: String,
    pub message: String,
    pub open: bool,
    pub on_close: Option<fn()>,
}

impl From<Info> for InfoModal {
    fn from(i: Info) -> Self {
        Self {
            title: i.title,
            message: i.message,
            open: true,
            on_close: i.callback,
        }
    }
}

impl From<&Info> for InfoModal {
    fn from(i: &Info) -> Self {
        InfoModal {
            title: i.title.clone(),
            message: i.message.clone(),
            open: true,
            on_close: i.callback,
        }
    }
}
