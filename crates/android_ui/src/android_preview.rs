use super::*;
use android_tools::{kotlin, preview};
use workspace::SplitDirection;

impl AndroidPanel {
    pub(super) fn generate_preview(
        &mut self,
        target: AndroidTarget,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let root = match self.trusted_root(cx) {
            Ok(root) => root,
            Err(error) => {
                self.fail(error, window, cx);
                return;
            }
        };
        self.running = true;
        self.status = "Rendering Compose preview…".into();
        let selected = self.selected_preview.clone();
        let executor = cx.background_executor().clone();
        self.preview_task = Some(cx.spawn_in(window, async move |panel, cx| {
            let result = cx.background_spawn({
                let root = root.clone();
                async move {
                    let installation = preview::installation()?;
                    let java = kotlin::java_home()?.join("bin/java");
                    let cache = preview::prepare(&root)?;
                    let temporary = tempfile::Builder::new().prefix("render-").tempdir_in(&cache)?;
                    let directory = temporary.path();
                    let program = if cfg!(windows) { root.join("gradlew.bat") } else { PathBuf::from("/bin/sh") };
                    let mut args = if cfg!(windows) { Vec::new() } else { vec!["./gradlew".into()] };
                    args.extend(["--init-script".into(), cache.join("export.gradle").to_string_lossy().into_owned(),
                        format!("-Dzed.android.module={}", target.module), format!("-Dzed.android.variant={}", target.variant),
                        format!("{}:{}", target.module.trim_end_matches(':'), preview::MODEL_TASK),
                        "--no-configuration-cache".into(), "--console=plain".into()]);
                    let output = tool_output(program, args, &root, &executor, Duration::from_secs(300)).await?;
                    let model = preview::parse_model(&output, &root, &target)?;
                    let model_path = directory.join("model.json");
                    std::fs::write(&model_path, serde_json::to_vec(&model)?)?;
                    let previews_path = directory.join("previews.json");
                    tool_output(java.clone(), preview::bridge_arguments(&installation, "discover", &model_path, Some(&previews_path))?, &root, &executor, Duration::from_secs(60)).await?;
                    let previews = preview::read_previews(&previews_path)?;
                    let selected = previews.iter().find(|preview| Some(&preview.id) == selected.as_ref()).or_else(|| previews.first()).context("No Compose previews")?;
                    let apk = target.apk_paths()?.into_iter().next().context("The build produced no APK")?;
                    let settings = preview::render_settings(&model, selected, &apk, &installation, directory);
                    let settings_path = directory.join("render.json");
                    std::fs::write(&settings_path, serde_json::to_vec(&settings)?)?;
                    tool_output(java, preview::bridge_arguments(&installation, "render", &settings_path, None)?, &root, &executor, Duration::from_secs(120)).await?;
                    let image = preview::rendered_image(directory)?;
                    let final_image = cache.join("preview.png");
                    let file = tempfile::NamedTempFile::new_in(&cache)?;
                    std::fs::copy(image, file.path())?;
                    file.persist(&final_image)?;
                    Ok::<_, anyhow::Error>((final_image, selected.id.clone(), previews))
                }
            }).await;
            let opened = panel.update_in(cx, |panel, window, cx| {
                panel.running = false;
                let result = result.and_then(|(image, selected, previews)| {
                    ensure!(panel.trusted_root(cx)? == root, "The Android project changed during preview rendering");
                    panel.previews = previews;
                    panel.selected_preview = Some(selected);
                    panel.status = "Compose preview updated. Refresh after editing code or changing the build variant.".into();
                    let project_path = Workspace::project_path_for_path(panel.project.clone(), &image, false, cx);
                    let workspace = panel.workspace.clone();
                    Ok(cx.spawn_in(window, async move |_, cx| {
                        let (_, path) = project_path.await?;
                        workspace.update_in(cx, |workspace, window, cx| {
                            let pane = workspace.panes().iter()
                                .find(|pane| pane.read(cx).item_for_path(path.clone(), cx).is_some()).cloned()
                                .or_else(|| workspace.panes().iter().find(|pane| *pane != workspace.active_pane()).cloned())
                                .unwrap_or_else(|| workspace.split_pane(workspace.active_pane().clone(), SplitDirection::Right, window, cx));
                            workspace.open_path(path, Some(pane.downgrade()), true, window, cx)
                        })?.await?;
                        Ok::<_, anyhow::Error>(())
                    }))
                });
                cx.notify();
                match result { Ok(task) => Some(task), Err(error) => { panel.fail(error, window, cx); None } }
            }).log_err().flatten();
            if let Some(opened) = opened && let Err(error) = opened.await {
                panel.update_in(cx, |panel, window, cx| panel.fail(error, window, cx)).log_err();
            }
        }));
        cx.notify();
    }

    pub(super) fn preview_picker(&self, cx: &Context<Self>) -> impl IntoElement {
        let previews = self.previews.clone();
        let selected = self.selected_preview.clone();
        let panel = cx.weak_entity();
        PopoverMenu::new("compose-preview-picker")
            .trigger(
                Button::new("compose-preview-select", "Select preview…")
                    .disabled(self.running || self.syncing || previews.is_empty())
                    .tab_index(0isize)
                    .end_icon(Icon::new(IconName::ChevronDown).size(IconSize::XSmall)),
            )
            .menu(move |window, cx| {
                Some(ContextMenu::build(window, cx, |mut menu, _, _| {
                    for preview in &previews {
                        let panel = panel.clone();
                        let id = preview.id.clone();
                        menu = menu.entry(
                            format!(
                                "{}{}",
                                if Some(&id) == selected.as_ref() {
                                    "✓ "
                                } else {
                                    ""
                                },
                                preview.label()
                            ),
                            None,
                            move |window, cx| {
                                panel
                                    .update(cx, |panel, cx| {
                                        panel.selected_preview = Some(id.clone());
                                        panel.gradle(GradleOperation::Preview, window, cx);
                                    })
                                    .log_err();
                            },
                        );
                    }
                    menu
                }))
            })
    }
}
