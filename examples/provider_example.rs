use async_walkdir::WalkDir;
use file_icon_provider::Provider;
use iced::{
    Element, Length, Subscription,
    alignment::Vertical,
    futures::{SinkExt, Stream, StreamExt, channel::mpsc::Sender},
    stream,
    widget::{Column, image, row, scrollable, text},
};
use std::path::PathBuf;

#[derive(Debug)]
enum Message {
    NewFileFound(PathBuf),
}

struct File {
    path: PathBuf,
    icon: image::Handle,
}

struct ProviderExample {
    provider: Provider<image::Handle>,
    files: Vec<File>,
}

impl ProviderExample {
    fn update(&mut self, message: Message) {
        match message {
            Message::NewFileFound(path) => {
                if let Ok(icon) = self.provider.get_file_icon(&path) {
                    self.files.push(File { path, icon })
                }
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        scrollable(Column::from_iter(self.files.iter().map(Self::file_view)).width(Length::Fill))
            .into()
    }

    fn file_view(file: &File) -> Element<'_, Message> {
        row![
            image(file.icon.clone()).filter_method(image::FilterMethod::Nearest),
            text(file.path.display().to_string()).wrapping(text::Wrapping::None)
        ]
        .spacing(4)
        .align_y(Vertical::Center)
        .into()
    }

    fn subscription(&self) -> Subscription<Message> {
        Subscription::run(discover_filesystem)
    }
}

fn discover_filesystem() -> impl Stream<Item = Message> {
    stream::channel(100, |mut output: Sender<Message>| async move {
        #[cfg(target_os = "windows")]
        let mut entries = WalkDir::new("C:\\");
        #[cfg(not(target_os = "windows"))]
        let mut entries = WalkDir::new("/");

        for _ in 0..1000 {
            match entries.next().await {
                Some(Ok(entry)) => {
                    output
                        .send(Message::NewFileFound(entry.path()))
                        .await
                        .unwrap();
                }
                Some(Err(error)) => {
                    println!("Error: {}", error);
                }
                None => break,
            }
        }
    })
}

impl Default for ProviderExample {
    fn default() -> Self {
        Self {
            provider: Provider::new(16, |icon| {
                image::Handle::from_rgba(icon.width, icon.height, icon.pixels)
            })
            .expect("create Provider"),
            files: Vec::new(),
        }
    }
}

fn main() -> iced::Result {
    iced::application(
        ProviderExample::default,
        ProviderExample::update,
        ProviderExample::view,
    )
    .subscription(ProviderExample::subscription)
    .run()
}
