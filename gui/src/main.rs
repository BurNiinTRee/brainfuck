use druid::{
    widget::{Align, Button, Flex, Label},
    AppDelegate, AppLauncher, Command, Data, DelegateCtx, Env, FileDialogOptions, Lens,
    LocalizedString, Target, Widget, WidgetExt, WindowDesc,
};

#[derive(Clone, Data, Debug, Lens)]
struct HelloState {
    file_name: String,
}

struct Delegater;
impl AppDelegate<HelloState> for Delegater {
    fn command(
        &mut self,
        ctx: &mut DelegateCtx<'_>,
        target: Target,
        cmd: &Command,
        data: &mut HelloState,
        env: &Env,
    ) -> bool {
        if let Some(info) = cmd.get(druid::commands::OPEN_FILE) {
            data.file_name = info.path().to_string_lossy().into_owned();
            false
        } else {
            true
        }
    }
}

fn main() {
    let main_window = WindowDesc::new(build_root_widget)
        .title("Hello World")
        .window_size((400.0, 400.0));

    let initial_state = HelloState {
        file_name: "World".into(),
    };

    AppLauncher::with_window(main_window)
        .delegate(Delegater)
        .launch(initial_state)
        .expect("Failed to launch application");
}

fn build_root_widget() -> impl Widget<HelloState> {
    let label = Label::new(|data: &HelloState, _env: &Env| format!("Hello {}!", data.file_name));

    let file_opener = Button::new("Open File").on_click(|ctx, state: &mut HelloState, _env| {
        ctx.submit_command(
            druid::commands::SHOW_OPEN_PANEL.with(FileDialogOptions::new()),
            None,
        );
    });

    let layout = Flex::column()
        .with_child(label)
        .with_spacer(20.0)
        .with_child(file_opener);

    Align::centered(layout)
}
