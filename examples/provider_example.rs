use std::path::PathBuf;

use file_icon_provider::FileIconProvider;
use iced::{
    alignment::Vertical,
    widget::{button, container, image, row, scrollable, text, Column},
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
    }

    Task::none()
}

fn view(state: &State) -> Element<Message> {
    let children = state
        .paths
        .iter()
        .map(|path| {
            row![
                image(state.file_icon_provider.icon(path).expect("Icon for file"))
                    .width(16)
                    .height(16)
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

    container(scrollable(Column::with_children(children))).into()
}

async fn add_files() -> Option<Vec<PathBuf>> {
    rfd::AsyncFileDialog::new()
        .pick_files()
        .await
        .map(|files| {
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
}

struct State {
    paths: Vec<PathBuf>,
    file_icon_provider: FileIconProvider<image::Handle>,
}

impl Default for State {
    fn default() -> Self {
        Self {
            file_icon_provider: FileIconProvider::new(|icon| {
                image::Handle::from_rgba(icon.width, icon.height, icon.pixels)
            }),
            paths: Vec::new(),
        }
    }
}
