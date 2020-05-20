fn main() {
    <HotswapTest as iced::Application>::run(iced::Settings::default());
}

struct HotswapTest {
    // Note: One can use without hotswapping
    // layout_file: iml::ImlFile<State>,
    hotswappo: Hotswappo,
}

#[derive(Default, Debug, Clone, serde::Serialize)]
struct State {
    clicked: bool,
    some_message: String,
    slider_val: f32,
}

#[derive(Debug, Copy, Clone, serde::Deserialize)]
enum Message {
    Testaroo,
    TestarooTwo(f32),
}

#[derive(Debug, Copy, Clone)]
enum AppMessage {
    UserMessage(Message),
    PollFileChanges(std::time::Instant),
}

impl iced::Application for HotswapTest {
    type Executor = iced::executor::Default;
    type Message = AppMessage;
    type Flags = ();

    fn new(_flags: Self::Flags) -> (Self, iced::Command<Self::Message>) {
        let layout_file_path =
            std::path::PathBuf::from(concat!(env!("CARGO_MANIFEST_DIR"), "/src/simple.ron"));

        (
            Self {
                hotswappo: Hotswappo::new(layout_file_path),
            },
            iced::Command::none(),
        )
    }

    fn title(&self) -> String {
        format!("iml loader, reload #{}", self.hotswappo.reload_count)
    }

    fn update(&mut self, message: Self::Message) -> iced::Command<Self::Message> {
        match message {
            AppMessage::UserMessage(message) => match message {
                Message::Testaroo => {
                    self.hotswappo.state().clicked = !self.hotswappo.state().clicked;
                }
                Message::TestarooTwo(val) => {
                    self.hotswappo.state().some_message = format!("{}", val);
                    self.hotswappo.state().slider_val = val;
                }
            },
            AppMessage::PollFileChanges(_) => {
                // no-op
            }
        }

        iced::Command::none()
    }

    fn view(&mut self) -> iced::Element<Self::Message> {
        self.hotswappo
            .get()
            .unwrap()
            .view::<Message>()
            .map(AppMessage::UserMessage)
    }

    fn subscription(&self) -> iced::Subscription<Self::Message> {
        self.hotswappo
            .subscription()
            .map(AppMessage::PollFileChanges)
    }
}

/// Container for hot swapping the layout file
struct Hotswappo {
    path: std::path::PathBuf,
    last_update: Option<std::time::SystemTime>,
    reload_count: usize,

    layout: Option<iml::ImlFile<State>>,
}

#[derive(Debug)]
enum HotswappoError {
    Io(std::io::Error),
}

impl From<std::io::Error> for HotswappoError {
    fn from(err: std::io::Error) -> Self {
        HotswappoError::Io(err)
    }
}

impl Hotswappo {
    pub fn new(path: std::path::PathBuf) -> Self {
        dbg!(&path);
        Self {
            path,
            last_update: None,
            reload_count: 0,

            layout: None,
        }
    }

    pub fn subscription(&self) -> iced::Subscription<std::time::Instant> {
        // Wake the app every 500 milliseconds to force a rerender and pick up file changes.
        // TODO listen to file update events
        iced::Subscription::from_recipe(Every(std::time::Duration::from_millis(500)))
    }

    pub fn state(&mut self) -> &mut State {
        self.get().unwrap().state()
    }

    /// This simply reloads the file if the modified time is more recent than the last modified time. It access the file on ever access.
    pub fn get(&mut self) -> Result<&mut iml::ImlFile<State>, HotswappoError> {
        let modified_time = self.path.metadata()?.modified()?;
        if self.last_update.is_none() || self.last_update.unwrap() < modified_time {
            let old_state = self
                .layout
                .as_mut()
                .map(|l| l.state().clone())
                .unwrap_or_default();
            self.layout = Some(iml::ImlFile::load_iml(
                std::fs::File::open(&self.path)?,
                old_state,
            )?);
            self.last_update = Some(modified_time);
            self.reload_count += 1;
        }

        Ok(self.layout.as_mut().unwrap())
    }
}

struct Every(std::time::Duration);

impl<H, I> iced_native::subscription::Recipe<H, I> for Every
where
    H: std::hash::Hasher,
{
    type Output = std::time::Instant;

    fn hash(&self, state: &mut H) {
        use std::hash::Hash;
        std::any::TypeId::of::<Self>().hash(state);
    }

    fn stream(
        self: Box<Self>,
        _input: futures::stream::BoxStream<'static, I>,
    ) -> futures::stream::BoxStream<'static, Self::Output> {
        use futures::stream::StreamExt;

        async_std::stream::interval(self.0)
            .map(|_| std::time::Instant::now())
            .boxed()
    }
}
