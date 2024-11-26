use std::path::PathBuf;

use file_icon_provider::FileIconProvider;
use iced::{
    alignment::Vertical,
    widget::{button, column, container, image, row, scrollable, slider, text, Column},
    Element, Length, Task,
};

fn main() -> iced::Result {
    iced::run("File Icon Provider Example", update, view)
}

fn update(state: &mut State, message: Message) -> Task<Message> {
    match message {
        Message::AddFiles => return Task::perform(add_files(), Message::NewFiles),
        Message::NewFiles(Some(mut paths)) => state.paths.append(&mut paths),
        Message::NewFiles(None) => (),
        Message::IconSizeChanged(icon_size) => {
            state.icon_size = icon_size;
            state.file_icon_provider.clear();
        }
    }

    Task::none()
}

fn view(state: &State) -> Element<Message> {
    let children = state
        .paths
        .iter()
        .map(|path| {
            row![
                image(
                    state
                        .file_icon_provider
                        .icon(path, state.icon_size)
                        .expect("Icon for file")
                )
                .filter_method(image::FilterMethod::Nearest),
                text(path.display().to_string()).wrapping(text::Wrapping::None)
            ]
            .spacing(4)
            .align_y(Vertical::Center)
            .into()
        })
        .chain(std::iter::once(
            container(button("Add Files...").on_press(Message::AddFiles))
                .padding(8)
                .center_x(Length::Fill)
                .into(),
        ));

    column![
        row![
            text("Icon size:"),
            slider(1..=512, state.icon_size, Message::IconSizeChanged),
            text!("{}px", state.icon_size)
        ]
        .align_y(Vertical::Center)
        .padding(4)
        .spacing(4),
        container(scrollable(Column::with_children(children)))
    ]
    .into()
}

async fn add_files() -> Option<Vec<PathBuf>> {
    rfd::AsyncFileDialog::new().pick_files().await.map(|files| {
        files
            .into_iter()
            .map(|fh| fh.path().to_path_buf())
            .collect()
    })
}

#[derive(Debug, Clone)]
enum Message {
    AddFiles,
    NewFiles(Option<Vec<PathBuf>>),
    IconSizeChanged(u16),
}

struct State {
    paths: Vec<PathBuf>,
    file_icon_provider: FileIconProvider<image::Handle>,
    icon_size: u16,
}

impl Default for State {
    fn default() -> Self {
        Self {
            file_icon_provider: FileIconProvider::new(|icon| {
                image::Handle::from_rgba(icon.width, icon.height, icon.pixels)
            }),
            paths: Vec::new(),
            icon_size: 16,
        }
    }
}
