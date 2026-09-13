use android_tools::{
    AndroidTarget, Device, adb_path, android_cli_path, is_gradle_project, parse_devices,
    parse_targets,
};
use anyhow::{Context as _, Result, bail, ensure};
use db::kvp::KeyValueStore;
use futures::future::{Either, select};
use gpui::{
    Action, App, BackgroundExecutor, Context, Entity, EventEmitter, FocusHandle, Focusable,
    Subscription, Task, WeakEntity, actions,
};
use project::{Project, TaskSourceKind, trusted_worktrees::TrustedWorktrees};
use settings::{IntoGpui, RegisterSetting, Settings};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use task::{RevealStrategy, SaveStrategy, TaskContext, TaskTemplate};
use ui::{ContextMenu, PopoverMenu, Tooltip, prelude::*};
use util::{ResultExt as _, command::new_command};
use workspace::{
    Workspace,
    dock::{DockPosition, Panel, PanelEvent},
    tasks::ScheduledTaskResult,
};

actions!(
    android,
    [
        /// Opens the Android project and device tools.
        ToggleFocus,
        /// Evaluates the Android project's modules and build variants.
        SyncProject,
        /// Refreshes connected Android devices.
        RefreshDevices,
        /// Builds the selected Android variant.
        Build,
        /// Builds and launches the selected Android variant on the selected device.
        Run,
        /// Runs local unit tests for the selected Android variant.
        Test,
        /// Runs Android lint for the selected variant.
        Lint,
        /// Opens Logcat for the selected Android device.
        Logcat,
    ]
);

pub fn init(cx: &mut App) {
    cx.observe_new(|workspace: &mut Workspace, window, cx| {
        let Some(window) = window else { return };
        let panel = cx
            .new(|cx| AndroidPanel::new(workspace.weak_handle(), workspace.project().clone(), cx));
        workspace.add_panel(panel, window, cx);
        workspace
            .register_action(|workspace, _: &ToggleFocus, window, cx| {
                workspace.toggle_panel_focus::<AndroidPanel>(window, cx);
            })
            .register_action(|workspace, _: &SyncProject, window, cx| {
                with_panel(workspace, window, cx, AndroidPanel::sync_project)
            })
            .register_action(|workspace, _: &RefreshDevices, window, cx| {
                with_panel(workspace, window, cx, |panel, _, cx| {
                    panel.refresh_devices(cx)
                })
            })
            .register_action(|workspace, _: &Build, window, cx| {
                with_panel(workspace, window, cx, |panel, window, cx| {
                    panel.gradle(GradleOperation::Build, window, cx)
                })
            })
            .register_action(|workspace, _: &Run, window, cx| {
                with_panel(workspace, window, cx, |panel, window, cx| {
                    panel.gradle(GradleOperation::Run, window, cx)
                })
            })
            .register_action(|workspace, _: &Test, window, cx| {
                with_panel(workspace, window, cx, |panel, window, cx| {
                    panel.gradle(GradleOperation::Test, window, cx)
                })
            })
            .register_action(|workspace, _: &Lint, window, cx| {
                with_panel(workspace, window, cx, |panel, window, cx| {
                    panel.gradle(GradleOperation::Lint, window, cx)
                })
            })
            .register_action(|workspace, _: &Logcat, window, cx| {
                with_panel(workspace, window, cx, AndroidPanel::logcat)
            });
        cx.notify();
    })
    .detach();
}

fn with_panel(
    workspace: &Workspace,
    window: &mut Window,
    cx: &mut Context<Workspace>,
    callback: impl FnOnce(&mut AndroidPanel, &mut Window, &mut Context<AndroidPanel>) + 'static,
) {
    if let Some(panel) = workspace.panel::<AndroidPanel>(cx) {
        // Task scheduling updates the workspace, so wait until its action handler has returned.
        window.defer(cx, move |window, cx| {
            panel.update(cx, |panel, cx| callback(panel, window, cx))
        });
    }
}

pub fn toolbar(workspace: &WeakEntity<Workspace>, cx: &App) -> Option<Entity<AndroidToolbar>> {
    let workspace = workspace.upgrade()?;
    let panel = workspace.read(cx).panel::<AndroidPanel>(cx)?;
    Some(panel.read(cx).toolbar.clone())
}

#[derive(Clone, Copy)]
enum GradleOperation {
    Build,
    Run,
    Test,
    Lint,
}

#[derive(RegisterSetting)]
struct AndroidPanelSettings {
    button: bool,
    dock: DockPosition,
    default_width: Pixels,
}

impl Settings for AndroidPanelSettings {
    fn from_settings(content: &settings::SettingsContent) -> Self {
        let panel = content.android_panel.clone().unwrap_or_default();
        Self {
            button: panel.button.unwrap_or(true),
            dock: panel.dock.unwrap_or(settings::DockPosition::Right).into(),
            default_width: panel
                .default_width
                .map(|width| width.into_gpui())
                .unwrap_or(px(320.)),
        }
    }
}

pub struct AndroidPanel {
    workspace: WeakEntity<Workspace>,
    project: Entity<Project>,
    toolbar: Entity<AndroidToolbar>,
    focus_handle: FocusHandle,
    root: Option<PathBuf>,
    targets: Vec<AndroidTarget>,
    selected_target: Option<AndroidTarget>,
    devices: Vec<Device>,
    selected_serial: Option<String>,
    status: SharedString,
    error: Option<String>,
    device_error: Option<String>,
    syncing: bool,
    refreshing_devices: bool,
    running: bool,
    sync_task: Option<Task<()>>,
    device_task: Option<Task<()>>,
    deploy_task: Option<Task<()>>,
}

impl AndroidPanel {
    fn new(
        workspace: WeakEntity<Workspace>,
        project: Entity<Project>,
        cx: &mut Context<Self>,
    ) -> Self {
        let panel = cx.entity();
        let toolbar = cx.new(|cx| AndroidToolbar {
            panel: panel.downgrade(),
            _subscription: cx.observe(&panel, |_, _, cx| cx.notify()),
        });
        Self {
            workspace,
            project,
            toolbar,
            focus_handle: cx.focus_handle(),
            root: None,
            targets: Vec::new(),
            selected_target: None,
            devices: Vec::new(),
            selected_serial: None,
            status: "Sync an Android project to discover its build variants.".into(),
            error: None,
            device_error: None,
            syncing: false,
            refreshing_devices: false,
            running: false,
            sync_task: None,
            device_task: None,
            deploy_task: None,
        }
    }

    fn roots(&self, cx: &App) -> Vec<PathBuf> {
        self.project
            .read(cx)
            .visible_worktrees(cx)
            .map(|worktree| worktree.read(cx).abs_path().to_path_buf())
            .collect()
    }

    fn trusted_root(&self, cx: &App) -> Result<PathBuf> {
        let project = self.project.read(cx);
        ensure!(
            project.is_local(),
            "Android tools currently support local projects only."
        );
        ensure!(
            !TrustedWorktrees::has_restricted_worktrees(&project.worktree_store(), cx),
            "Trust the project using the title bar's Restricted Mode control before running Android tools."
        );
        if let Some(root) = &self.root {
            ensure!(
                self.roots(cx).contains(root),
                "The selected project is no longer open."
            );
            return Ok(root.clone());
        }
        let roots = self.roots(cx);
        ensure!(!roots.is_empty(), "Open an Android Gradle project first.");
        ensure!(
            roots.len() == 1,
            "Select a project folder in the Android panel before syncing."
        );
        roots
            .into_iter()
            .next()
            .context("Open an Android Gradle project first.")
    }

    fn fail(&mut self, error: anyhow::Error, window: &mut Window, cx: &mut Context<Self>) {
        self.error = Some(format!("{error:#}"));
        self.status = "Android operation failed".into();
        let workspace = self.workspace.clone();
        window.defer(cx, move |window, cx| {
            workspace
                .update(cx, |workspace, cx| {
                    workspace.open_panel::<AndroidPanel>(window, cx)
                })
                .log_err();
        });
        cx.notify();
    }

    fn sync_project(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.syncing || self.running {
            return;
        }
        let root = match self.trusted_root(cx) {
            Ok(root) => root,
            Err(error) => {
                self.fail(error, window, cx);
                return;
            }
        };
        self.root = Some(root.clone());
        self.syncing = true;
        self.targets.clear();
        self.error = None;
        self.status = "Syncing Android project…".into();
        self.refresh_devices(cx);
        let executor = cx.background_executor().clone();
        self.sync_task = Some(cx.spawn_in(window, async move |panel, cx| {
            let result = cx
                .background_spawn(async move {
                    ensure!(
                        is_gradle_project(&root),
                        "This folder has no Gradle wrapper. Open the project's Gradle root."
                    );
                    let output = tool_output(
                        android_cli_path()?,
                        vec![
                            "describe".into(),
                            format!("--project_dir={}", root.display()),
                        ],
                        &root,
                        &executor,
                        Duration::from_secs(300),
                    )
                    .await?;
                    parse_targets(&output)
                })
                .await;
            panel
                .update_in(cx, |panel, window, cx| {
                    panel.syncing = false;
                    match result {
                        Ok(targets) => {
                            panel.apply_targets(targets, cx);
                            cx.notify();
                        }
                        Err(error) => {
                            panel.selected_target = None;
                            panel.fail(error, window, cx);
                        }
                    }
                })
                .log_err();
        }));
        cx.notify();
    }

    fn refresh_devices(&mut self, cx: &mut Context<Self>) {
        if self.refreshing_devices {
            return;
        }
        self.refreshing_devices = true;
        let executor = cx.background_executor().clone();
        self.device_task = Some(cx.spawn(async move |panel, cx| {
            let result = cx
                .background_spawn(async move {
                    let output = tool_output(
                        adb_path()?,
                        vec!["devices".into(), "-l".into()],
                        Path::new("."),
                        &executor,
                        Duration::from_secs(15),
                    )
                    .await?;
                    parse_devices(&output)
                })
                .await;
            panel
                .update(cx, |panel, cx| {
                    panel.refreshing_devices = false;
                    match result {
                        Ok(devices) => {
                            panel.device_error = None;
                            if panel.selected_serial.is_none() {
                                let available = devices
                                    .iter()
                                    .filter(|device| device.is_available())
                                    .collect::<Vec<_>>();
                                if available.len() == 1 {
                                    panel.selected_serial =
                                        available.first().map(|device| device.serial.clone());
                                }
                            }
                            panel.devices = devices;
                        }
                        Err(error) => {
                            panel.devices.clear();
                            panel.device_error = Some(format!("{error:#}"));
                        }
                    }
                    cx.notify();
                })
                .log_err();
        }));
        cx.notify();
    }

    fn selected_device(&self) -> Result<&Device> {
        self.devices.iter().find(|device| Some(&device.serial) == self.selected_serial.as_ref() && device.is_available())
            .context("Select an available Android device. Start an emulator or connect a device, then refresh the device list.")
    }

    fn gradle(&mut self, operation: GradleOperation, window: &mut Window, cx: &mut Context<Self>) {
        if self.running || self.syncing {
            return;
        }
        let result = (|| {
            let root = self.trusted_root(cx)?;
            let target = self
                .selected_target
                .clone()
                .context("Sync the Android project and select a build variant first.")?;
            let run_after = if matches!(operation, GradleOperation::Run) {
                Some((target.clone(), self.selected_device()?.serial.clone()))
            } else {
                None
            };
            let (name, gradle_task) = match operation {
                GradleOperation::Build | GradleOperation::Run => {
                    ("Build", target.gradle_task("assemble", ""))
                }
                GradleOperation::Test => ("Test", target.gradle_task("test", "UnitTest")),
                GradleOperation::Lint => ("Lint", target.gradle_task("lint", "")),
            };
            let (program, args) = if cfg!(windows) {
                (
                    root.join("gradlew.bat"),
                    vec![gradle_task, "--console=plain".into()],
                )
            } else {
                (
                    PathBuf::from("/bin/sh"),
                    vec!["./gradlew".into(), gradle_task, "--console=plain".into()],
                )
            };
            self.schedule(
                format!("Android {name} · {}", target.label()),
                program,
                args,
                root,
                run_after,
                window,
                cx,
            )
        })();
        if let Err(error) = result {
            self.fail(error, window, cx);
        }
    }

    fn schedule(
        &mut self,
        label: String,
        program: PathBuf,
        args: Vec<String>,
        root: PathBuf,
        run_after: Option<(AndroidTarget, String)>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Result<()> {
        let template = TaskTemplate {
            label: label.clone(),
            command: program.to_string_lossy().into_owned(),
            args,
            reveal: RevealStrategy::NoFocus,
            save: SaveStrategy::All,
            show_summary: true,
            show_command: true,
            ..Default::default()
        };
        let task = template.resolve_task("android", &TaskContext { cwd: Some(root), ..Default::default() })
            .context("Could not resolve the Android command. Check the project path and tool configuration.")?;
        let panel = cx.weak_entity();
        self.workspace.update(cx, |workspace, cx| {
            workspace.schedule_resolved_task_with_completion(TaskSourceKind::UserInput, task, false, move |result, cx| {
                panel.update_in(cx, |panel, window, cx| {
                    panel.running = false;
                    match result {
                        ScheduledTaskResult::Success => {
                            panel.status = "Task completed successfully".into();
                            if let Some((target, serial)) = run_after { panel.deploy(target, serial, window, cx); }
                        }
                        ScheduledTaskResult::Cancelled => panel.status = "Task cancelled".into(),
                        ScheduledTaskResult::Failure | ScheduledTaskResult::SpawnFailed => {
                            panel.fail(anyhow::anyhow!("Android command failed. See the task terminal for the error and retry after fixing it."), window, cx);
                        }
                    }
                    cx.notify();
                }).log_err();
            }, window, cx);
        })?;
        self.running = true;
        self.error = None;
        self.status = label.into();
        cx.notify();
        Ok(())
    }

    fn deploy(
        &mut self,
        target: AndroidTarget,
        serial: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.running = true;
        self.status = "Preparing APK for deployment…".into();
        let root = self.root.clone();
        self.deploy_task = Some(cx.spawn_in(window, async move |panel, cx| {
            let result = cx
                .background_spawn(async move {
                    let paths = target.apk_paths()?;
                    Ok::<_, anyhow::Error>((
                        android_cli_path()?,
                        paths
                            .into_iter()
                            .map(|path| path.to_string_lossy().into_owned())
                            .collect::<Vec<_>>()
                            .join(","),
                    ))
                })
                .await;
            panel
                .update_in(cx, |panel, window, cx| {
                    panel.running = false;
                    let result = result.and_then(|(program, apks)| {
                        let root = root.context("The Android project was closed.")?;
                        ensure!(
                            panel.trusted_root(cx)? == root,
                            "The selected Android project changed during the build."
                        );
                        panel.schedule(
                            "Android Run".into(),
                            program,
                            vec![
                                "run".into(),
                                format!("--device={serial}"),
                                format!("--apks={apks}"),
                            ],
                            root,
                            None,
                            window,
                            cx,
                        )
                    });
                    if let Err(error) = result {
                        panel.fail(error, window, cx);
                    }
                })
                .log_err();
        }));
    }

    fn logcat(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let result = (|| {
            let root = self.trusted_root(cx)?;
            let serial = self.selected_device()?.serial.clone();
            let template = TaskTemplate {
                label: format!("Logcat · {serial}"),
                command: adb_path()?.to_string_lossy().into_owned(),
                args: vec![
                    "-s".into(),
                    serial,
                    "logcat".into(),
                    "-v".into(),
                    "threadtime".into(),
                ],
                reveal: RevealStrategy::Always,
                show_summary: true,
                show_command: true,
                ..Default::default()
            };
            let task = template
                .resolve_task(
                    "android-logcat",
                    &TaskContext {
                        cwd: Some(root),
                        ..Default::default()
                    },
                )
                .context("Could not resolve the Logcat command.")?;
            self.workspace.update(cx, |workspace, cx| {
                workspace.schedule_resolved_task(TaskSourceKind::UserInput, task, false, window, cx)
            })?;
            Ok::<_, anyhow::Error>(())
        })();
        if let Err(error) = result {
            self.fail(error, window, cx);
        }
    }

    fn target_selection_key(&self) -> Option<String> {
        let root = serde_json::to_string(self.root.as_ref()?).log_err()?;
        Some(format!("android-selected-target:{root}"))
    }

    fn remember_target(&self, cx: &App) {
        let Some(key) = self.target_selection_key() else {
            return;
        };
        let Some(target) = &self.selected_target else {
            return;
        };
        let Some(value) = serde_json::to_string(&(&target.module, &target.variant)).log_err()
        else {
            return;
        };
        let database = KeyValueStore::global(cx);
        db::write_and_log(
            cx,
            move || async move { database.write_kvp(key, value).await },
        );
    }

    fn apply_targets(&mut self, targets: Vec<AndroidTarget>, cx: &App) {
        let preferred = self
            .selected_target
            .as_ref()
            .map(|target| (target.module.clone(), target.variant.clone()))
            .or_else(|| {
                let key = self.target_selection_key()?;
                let value = KeyValueStore::global(cx).read_kvp(&key).log_err()??;
                serde_json::from_str::<(String, String)>(&value).log_err()
            });
        // Restore only identity; build tasks and artifact paths must come from the fresh model.
        self.selected_target = if let Some((module, variant)) = &preferred {
            targets
                .iter()
                .find(|target| &target.module == module && &target.variant == variant)
                .cloned()
        } else {
            let mut debug_targets = targets.iter().filter(|target| target.variant == "debug");
            let debug_target = debug_targets.next();
            if debug_target.is_some() && debug_targets.next().is_none() {
                debug_target.cloned()
            } else if targets.len() == 1 {
                targets.first().cloned()
            } else {
                None
            }
        };
        self.status = if preferred.is_some() && self.selected_target.is_none() {
            "The previous build variant is unavailable. Select a build variant to continue.".into()
        } else {
            format!("Sync complete · {} build variants", targets.len()).into()
        };
        self.targets = targets;
    }

    fn target_picker(&self, id: &'static str, cx: &Context<Self>) -> impl IntoElement {
        let targets = self.targets.clone();
        let panel = cx.weak_entity();
        let label = self
            .selected_target
            .as_ref()
            .map(AndroidTarget::label)
            .unwrap_or_else(|| "Select build variant".into());
        PopoverMenu::new(id)
            .trigger(
                Button::new("target", label)
                    .label_size(LabelSize::Small)
                    .end_icon(Icon::new(IconName::ChevronDown).size(IconSize::XSmall))
                    .disabled(self.syncing || self.running || targets.is_empty())
                    .tab_index(0isize),
            )
            .menu(move |window, cx| {
                Some(ContextMenu::build(window, cx, |mut menu, _, _| {
                    for target in &targets {
                        let panel = panel.clone();
                        let target = target.clone();
                        menu = menu.entry(target.label(), None, move |_, cx| {
                            panel
                                .update(cx, |panel, cx| {
                                    panel.selected_target = Some(target.clone());
                                    panel.remember_target(cx);
                                    cx.notify();
                                })
                                .log_err();
                        });
                    }
                    menu
                }))
            })
    }

    fn device_picker(&self, id: &'static str, cx: &Context<Self>) -> impl IntoElement {
        let devices = self.devices.clone();
        let panel = cx.weak_entity();
        let label = self
            .selected_device()
            .map(|device| device.model.clone())
            .unwrap_or_else(|_| "Select device".into());
        PopoverMenu::new(id)
            .trigger(
                Button::new("device", label)
                    .label_size(LabelSize::Small)
                    .end_icon(Icon::new(IconName::ChevronDown).size(IconSize::XSmall))
                    .disabled(self.running || devices.is_empty())
                    .tab_index(0isize),
            )
            .menu(move |window, cx| {
                Some(ContextMenu::build(window, cx, |mut menu, _, _| {
                    for device in &devices {
                        let panel = panel.clone();
                        let device = device.clone();
                        let label =
                            format!("{} · {} · {}", device.model, device.serial, device.state);
                        menu = menu.entry(label, None, move |_, cx| {
                            panel
                                .update(cx, |panel, cx| {
                                    panel.selected_serial = Some(device.serial.clone());
                                    cx.notify();
                                })
                                .log_err();
                        });
                    }
                    menu
                }))
            })
    }

    fn render_toolbar(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .gap_1()
            .child(
                IconButton::new("android-tools", IconName::ToolHammer)
                    .tab_index(0isize)
                    .aria_label("Android tools")
                    .tooltip(|_, cx| Tooltip::for_action("Android", &ToggleFocus, cx))
                    .on_click(|_, window, cx| {
                        window.dispatch_action(ToggleFocus.boxed_clone(), cx)
                    }),
            )
            .child(self.target_picker("toolbar-target", cx))
            .child(self.device_picker("toolbar-device", cx))
            .child(
                IconButton::new("android-run", IconName::PlayFilled)
                    .tab_index(0isize)
                    .aria_label("Run app")
                    .icon_color(Color::Success)
                    .disabled(
                        self.running
                            || self.syncing
                            || self.selected_target.is_none()
                            || self.selected_device().is_err(),
                    )
                    .tooltip(|_, cx| Tooltip::for_action("Run app", &Run, cx))
                    .on_click(cx.listener(|panel, _, window, cx| {
                        panel.gradle(GradleOperation::Run, window, cx)
                    })),
            )
            .child(
                IconButton::new("android-build", IconName::ToolHammer)
                    .tab_index(0isize)
                    .aria_label("Build selected variant")
                    .disabled(self.running || self.syncing || self.selected_target.is_none())
                    .tooltip(|_, cx| Tooltip::for_action("Build selected variant", &Build, cx))
                    .on_click(cx.listener(|panel, _, window, cx| {
                        panel.gradle(GradleOperation::Build, window, cx)
                    })),
            )
            .child(
                IconButton::new("android-sync", IconName::RefreshTitle)
                    .tab_index(0isize)
                    .aria_label("Sync Android project")
                    .disabled(self.running || self.syncing)
                    .tooltip(|_, cx| Tooltip::for_action("Sync Android project", &SyncProject, cx))
                    .on_click(cx.listener(|panel, _, window, cx| panel.sync_project(window, cx))),
            )
    }
}

impl Render for AndroidPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let roots = self.roots(cx);
        let panel = cx.weak_entity();
        let root_label = self
            .root
            .as_ref()
            .or_else(|| {
                if roots.len() == 1 {
                    roots.first()
                } else {
                    None
                }
            })
            .and_then(|root| root.file_name())
            .map(|name| name.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Select project".into());
        let project_picker = PopoverMenu::new("android-project")
            .trigger(
                Button::new("project", root_label)
                    .disabled(self.running || self.syncing)
                    .tab_index(0isize),
            )
            .menu(move |window, cx| {
                Some(ContextMenu::build(window, cx, |mut menu, _, _| {
                    for root in &roots {
                        let panel = panel.clone();
                        let root = root.clone();
                        menu =
                            menu.entry(root.to_string_lossy().into_owned(), None, move |_, cx| {
                                panel
                                    .update(cx, |panel, cx| {
                                        panel.root = Some(root.clone());
                                        panel.targets.clear();
                                        panel.selected_target = None;
                                        panel.error = None;
                                        panel.status =
                                            "Sync this project to discover its build variants."
                                                .into();
                                        cx.notify();
                                    })
                                    .log_err();
                            });
                    }
                    menu
                }))
            });
        let commands = [
            (GradleOperation::Build, "Build"),
            (GradleOperation::Run, "Run"),
            (GradleOperation::Test, "Test"),
            (GradleOperation::Lint, "Lint"),
        ]
        .into_iter()
        .map(|(operation, label)| {
            let is_run = matches!(operation, GradleOperation::Run);
            Button::new(label, label)
                .when(is_run, |button| button.style(ButtonStyle::Filled))
                .disabled(
                    self.syncing
                        || self.running
                        || self.selected_target.is_none()
                        || (is_run && self.selected_device().is_err()),
                )
                .tab_index(0isize)
                .on_click(
                    cx.listener(move |panel, _, window, cx| panel.gradle(operation, window, cx)),
                )
        })
        .collect::<Vec<_>>();
        v_flex()
            .id("android-panel")
            .key_context("AndroidPanel")
            .track_focus(&self.focus_handle)
            .role(gpui::Role::Complementary)
            .aria_label("Android tools")
            .size_full()
            .p_3()
            .gap_3()
            .overflow_y_scroll()
            .child(h_flex().justify_between()
                .child(Label::new("Android").size(LabelSize::Large))
                .child(IconButton::new("close-android", IconName::Close)
                    .tab_index(0isize).aria_label("Hide Android tools")
                    .tooltip(Tooltip::text("Hide Android tools"))
                    .on_click(cx.listener(|_, _, _, cx| cx.emit(PanelEvent::Close)))))
            .child(Label::new("Project").color(Color::Muted))
            .child(project_picker)
            .child(Button::new("sync-project", if self.syncing { "Syncing…" } else { "Sync project" })
                .disabled(self.syncing || self.running).tab_index(0isize)
                .on_click(cx.listener(|panel, _, window, cx| panel.sync_project(window, cx))))
            .child(Label::new("Build variant").color(Color::Muted))
            .child(self.target_picker("panel-target", cx))
            .child(Label::new("Device").color(Color::Muted))
            .child(self.device_picker("panel-device", cx))
            .child(Button::new("refresh-devices", if self.refreshing_devices { "Refreshing…" } else { "Refresh devices" })
                .disabled(self.refreshing_devices).tab_index(0isize)
                .on_click(cx.listener(|panel, _, _, cx| panel.refresh_devices(cx))))
            .when(self.devices.is_empty(), |this| {
                this.child(div().text_sm().text_color(cx.theme().colors().text_muted)
                    .child("Start an Android emulator or connect a device with USB debugging, then refresh."))
            })
            .child(h_flex().flex_wrap().gap_1().children(commands))
            .child(Button::new("logcat", "Open Logcat")
                .disabled(self.selected_device().is_err()).tab_index(0isize)
                .on_click(cx.listener(|panel, _, window, cx| panel.logcat(window, cx))))
            .child(div().text_sm().text_color(cx.theme().colors().text_muted).child(self.status.clone()))
            .when_some(self.error.clone().or_else(|| self.device_error.clone()), |this, error| {
                this.child(div().text_sm().text_color(cx.theme().status().error).child(error))
            })
    }
}

impl EventEmitter<PanelEvent> for AndroidPanel {}
impl Focusable for AndroidPanel {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}
impl Panel for AndroidPanel {
    fn persistent_name() -> &'static str {
        "Android"
    }
    fn panel_key() -> &'static str {
        "android"
    }
    fn position(&self, _: &Window, cx: &App) -> DockPosition {
        AndroidPanelSettings::get_global(cx).dock
    }
    fn position_is_valid(&self, _: DockPosition) -> bool {
        true
    }
    fn set_position(&mut self, position: DockPosition, _: &mut Window, cx: &mut Context<Self>) {
        settings::update_settings_file(
            self.project.read(cx).fs().clone(),
            cx,
            move |settings, _| {
                settings.android_panel.get_or_insert_default().dock = Some(position.into());
            },
        );
    }
    fn default_size(&self, _: &Window, cx: &App) -> Pixels {
        AndroidPanelSettings::get_global(cx).default_width
    }
    fn icon(&self, _: &Window, cx: &App) -> Option<IconName> {
        AndroidPanelSettings::get_global(cx)
            .button
            .then_some(IconName::ToolHammer)
    }
    fn icon_tooltip(&self, _: &Window, _: &App) -> Option<&'static str> {
        Some("Android")
    }
    fn toggle_action(&self) -> Box<dyn gpui::Action> {
        Box::new(ToggleFocus)
    }
    fn activation_priority(&self) -> u32 {
        10
    }
    fn set_active(&mut self, active: bool, _: &mut Window, cx: &mut Context<Self>) {
        if active && self.devices.is_empty() {
            self.refresh_devices(cx);
        }
    }
}

pub struct AndroidToolbar {
    panel: WeakEntity<AndroidPanel>,
    _subscription: Subscription,
}
impl Render for AndroidToolbar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        div().children(
            self.panel
                .update(cx, |panel, cx| {
                    panel.render_toolbar(window, cx).into_any_element()
                })
                .log_err(),
        )
    }
}

async fn tool_output(
    program: PathBuf,
    args: Vec<String>,
    root: &Path,
    executor: &BackgroundExecutor,
    timeout: Duration,
) -> Result<String> {
    let mut command = new_command(&program);
    command.args(args).current_dir(root).kill_on_drop(true);
    let output = match select(
        Box::pin(command.output()),
        Box::pin(executor.timer(timeout)),
    )
    .await
    {
        Either::Left((output, _)) => {
            output.with_context(|| format!("Could not start {}", program.display()))?
        }
        Either::Right(_) => bail!(
            "{} timed out after {} seconds. Check the SDK, JDK, and network, then retry.",
            program.display(),
            timeout.as_secs()
        ),
    };
    let stdout = String::from_utf8_lossy(&output.stdout);
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let details = format!("{stdout}\n{stderr}");
        bail!(
            "{} failed ({}): {}",
            program.display(),
            output.status,
            details
                .chars()
                .rev()
                .take(4000)
                .collect::<String>()
                .chars()
                .rev()
                .collect::<String>()
        );
    }
    Ok(stdout.into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui::TestAppContext;
    use project::{
        FakeFs,
        trusted_worktrees::{self, PathTrust},
    };
    use serde_json::json;
    use workspace::AppState;

    #[gpui::test]
    async fn android_panel_respects_trust_roots_and_device_state(cx: &mut TestAppContext) {
        let _app_state = cx.update(|cx| {
            let state = AppState::test(cx);
            trusted_worktrees::init(Default::default(), cx);
            init(cx);
            state
        });
        cx.update(|cx| {
            for (platform, shortcuts) in [
                (
                    "macos",
                    [
                        ("cmd-f9", "android::Build"),
                        ("ctrl-r", "android::Run"),
                        ("cmd-6", "android::Logcat"),
                        ("cmd-alt-y", "android::SyncProject"),
                    ],
                ),
                (
                    "linux",
                    [
                        ("ctrl-f9", "android::Build"),
                        ("shift-f10", "android::Run"),
                        ("alt-6", "android::Logcat"),
                        ("ctrl-alt-y", "android::SyncProject"),
                    ],
                ),
            ] {
                let mut bindings = settings::KeymapFile::load_asset_allow_partial_failure(
                    &format!("keymaps/default-{platform}.json"),
                    cx,
                )
                .expect("Default keymap should load");
                for binding in &mut bindings {
                    binding.set_meta(settings::KeybindSource::Default.meta());
                }
                let mut studio = settings::KeymapFile::load_asset_allow_partial_failure(
                    &format!("keymaps/{platform}/jetbrains.json"),
                    cx,
                )
                .expect("JetBrains keymap should load");
                for binding in &mut studio {
                    binding.set_meta(settings::KeybindSource::Base.meta());
                }
                bindings.extend(studio);
                let keymap = gpui::Keymap::new(bindings);
                let contexts = ["Workspace", "Pane", "Editor mode=full"]
                    .map(|context| gpui::KeyContext::parse(context).expect("Valid key context"));
                for (shortcut, action) in shortcuts {
                    let keystroke = gpui::Keystroke::parse(shortcut).expect("Valid shortcut");
                    let (matches, _) = keymap.bindings_for_input(&[keystroke], &contexts);
                    assert_eq!(
                        matches.first().map(|binding| binding.action().name()),
                        Some(action),
                        "{platform}: {shortcut}"
                    );
                }
            }
        });
        let filesystem = FakeFs::new(cx.executor());
        filesystem
            .insert_tree(
                "/android",
                json!({"settings.gradle.kts": "", "gradlew": ""}),
            )
            .await;
        let project =
            Project::test_with_worktree_trust(filesystem.clone(), [Path::new("/android")], cx)
                .await;
        let (workspace, cx) =
            cx.add_window_view(|window, cx| Workspace::test_new(project.clone(), window, cx));
        cx.run_until_parked();
        let panel = workspace
            .read_with(cx, |workspace, cx| workspace.panel::<AndroidPanel>(cx))
            .expect("Android panel should register with a new workspace");
        panel.read_with(cx, |panel, cx| {
            assert!(panel.trusted_root(cx).is_err());
            assert!(panel.selected_device().is_err());
            assert!(!panel.running);
            assert!(!panel.syncing);
        });
        let store = project.read_with(cx, |project, _| project.worktree_store());
        cx.update(|_, cx| {
            TrustedWorktrees::try_get_global(cx)
                .expect("Trust store should exist")
                .update(cx, |trusted, cx| {
                    trusted.trust(
                        &store,
                        [PathTrust::AbsPath(PathBuf::from("/android"))]
                            .into_iter()
                            .collect(),
                        cx,
                    )
                });
        });
        panel.update(cx, |panel, cx| {
            assert_eq!(panel.trusted_root(cx).ok(), Some(PathBuf::from("/android")));
            panel.root = Some(PathBuf::from("/closed-project"));
            assert!(panel.trusted_root(cx).is_err());
            panel.root = None;
            panel.devices = parse_devices("List of devices attached\nemulator-1 offline\nemulator-2 device model:Test_Phone\n")
                .expect("Valid device list");
            panel.selected_serial = Some("emulator-1".into());
            assert!(panel.selected_device().is_err());
            panel.selected_serial = Some("emulator-2".into());
            assert_eq!(panel.selected_device().map(|device| device.serial.as_str()).ok(), Some("emulator-2"));
        });
        workspace.update_in(cx, |workspace, window, cx| {
            with_panel(workspace, window, cx, |panel, window, cx| {
                panel.gradle(GradleOperation::Build, window, cx)
            });
        });
        cx.run_until_parked();
        panel.read_with(cx, |panel, _| {
            assert!(!panel.running);
            assert!(
                panel
                    .error
                    .as_ref()
                    .is_some_and(|error| error.contains("select a build variant"))
            );
        });
        assert!(workspace.read_with(cx, |workspace, cx| {
            toolbar(&workspace.weak_handle(), cx).is_some()
        }));
        let (database, key) = panel.update(cx, |panel, cx| {
            panel.root = Some(PathBuf::from("/android"));
            (
                KeyValueStore::global(cx),
                panel.target_selection_key().expect("Project key"),
            )
        });
        database
            .write_kvp(
                key,
                serde_json::to_string(&(":mobile", "fullDebug")).expect("Target identity"),
            )
            .await
            .expect("Remember selected variant");
        panel.update(cx, |panel, cx| {
            let full = AndroidTarget {
                module: ":mobile".into(),
                variant: "fullDebug".into(),
                output_listing: PathBuf::from("/android/fresh/output.json"),
            };
            let demo = AndroidTarget {
                variant: "demoDebug".into(),
                ..full.clone()
            };
            panel.selected_target = None;
            panel.apply_targets(vec![demo.clone(), full.clone()], cx);
            assert_eq!(panel.selected_target, Some(full.clone()));
            let changed = AndroidTarget {
                output_listing: PathBuf::from("/android/new/output.json"),
                ..full
            };
            panel.apply_targets(vec![changed.clone()], cx);
            assert_eq!(panel.selected_target, Some(changed));
            panel.apply_targets(vec![demo], cx);
            assert!(panel.selected_target.is_none());
            assert!(panel.status.contains("unavailable"));
            panel.selected_target = None;
            panel.root = Some(PathBuf::from("/another-project"));
            let debug = AndroidTarget {
                module: ":app".into(),
                variant: "debug".into(),
                output_listing: PathBuf::from("/another-project/output.json"),
            };
            panel.apply_targets(vec![debug.clone()], cx);
            assert_eq!(panel.selected_target, Some(debug));
            panel.root = None;
        });
        panel.update_in(cx, |panel, window, cx| {
            panel.set_position(DockPosition::Left, window, cx)
        });
        cx.executor().advance_clock(Duration::from_millis(200));
        cx.run_until_parked();
        assert!(workspace.read_with(cx, |workspace, cx| {
            workspace
                .left_dock()
                .read(cx)
                .visible_panel()
                .is_some_and(|visible| visible.panel_id() == panel.entity_id())
        }));
        let settings_filesystem = project.read_with(cx, |project, _| project.fs().clone());
        let saved = settings::SettingsStore::load_settings(&settings_filesystem)
            .await
            .expect("Panel settings should persist");
        assert!(saved.contains("android_panel"));
        assert!(saved.contains("left"));
    }
}
