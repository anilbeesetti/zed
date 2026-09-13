# Local stack review guide

No branches have been pushed and no GitHub PRs have been created. These are
local `gh-stack` branches with review-ready descriptions. The current top is
`codex/android-ide/validation`; the base is local `main` at `7960b2a7c9568e90fbe0727332149e5b2a5fd57a`.

## Inspect and test the stack

```sh
gh stack view --json
git log --oneline main..codex/android-ide/validation
git diff --stat main..codex/android-ide/validation
```

For one layer, compare its branch against the parent in this table. This keeps
review focused without losing the runnable combined application at the top.
The actual Git branch tips are authoritative if a cached `gh-stack` head field
has not refreshed after a commit.

| Layer | Branch suffix under `codex/android-ide/` | Parent | Source tip before the report commit |
| --- | --- | --- | --- |
| 1 | `research` | `main` | `bbcbb5f406` |
| 2 | `studio-defaults` | `research` | `600b860c56` |
| 3 | `android-tools` | `studio-defaults` | `d0ff09aa75` |
| 4 | `android-workflow` | `android-tools` | `d79381e9f1` |
| 5 | `studio-shell` | `android-workflow` | `f46e27fbf1` |
| 6 | `kotlin-setup` | `studio-shell` | `a188c158ff` |
| 7 | `dev-launcher` | `kotlin-setup` | `972ba9dc74` |
| 8 | `emulator-start` | `dev-launcher` | `a9d4067e86` |
| 9 | `smoke-project` | `emulator-start` | `1651d598cc` |
| 10 | `validation` | `smoke-project` | This report commit |

For example:

```sh
git diff codex/android-ide/android-tools..codex/android-ide/android-workflow
git diff codex/android-ide/studio-shell..codex/android-ide/kotlin-setup
```

Fixes were committed to their owning layer and descendants rebased locally.
Future edits should follow the same approach: check out that layer, make and
validate the change, commit, then use `gh stack rebase --upstack --remote origin`
and return with `gh stack top`. A rebase may fetch its base; that is not a push.
Do not run stack push/submit or create PRs without new authorization.

Start with the [validation report](ANDROID_IDE_VALIDATION.md) for build commands,
supported workflows, known failures, performance caveats, and reproduction.
The root README notice is intentionally retained for human review. Do not let
an agent remove it while preparing a future submission.

## 1. Document the Android IDE architecture and validation plan

Branch: `codex/android-ide/research`

Describe the Android Studio migration target, reuse points in Zed, toolchain
prerequisites, language-server risks, phased implementation, and acceptance
gates. Add the required human-review notice to the root README.

Validation: inspect the cited official documentation and local source paths;
confirm the roadmap distinguishes implemented work from future parity goals.

Release Notes:

- N/A

## 2. Adopt Studio themes and JetBrains editing defaults

Branch: `codex/android-ide/studio-defaults`

Provide Studio Dark/Light themes and licensed JetBrains Mono fonts, choose
JetBrains keybindings by default, and adjust typography and initial tool-window
placement. Preserve user overrides. Add Android shortcut mappings with explicit
workspace and full-editor contexts so Sync is not intercepted by Git Stage.

Validation: native dark/light rendering, fresh-profile launch, deep Kotlin-file
discovery, and combined GPUI shortcut precedence tests for both keymap assets.

Release Notes:

- Added Studio-inspired appearance and familiar JetBrains editing defaults

## 3. Model Android projects, devices, and build artifacts

Branch: `codex/android-ide/android-tools`

Add a small non-UI crate for Gradle-root detection, SDK tool discovery, evaluated
Android targets, device states, and artifact selection. Commands identify the
project and device explicitly. Reject ambiguous or unsafe APK metadata paths
instead of guessing what to deploy.

Validation: focused parser/artifact tests cover malformed output, multiple
devices, module/flavor targets, metadata redirects, filters and missing files.
`cargo test -p android_tools` and `./script/clippy -p android_tools` pass.

Release Notes:

- Added Android project, device, and build-artifact discovery

## 4. Add native Android build and run controls

Branch: `codex/android-ide/android-workflow`

Add the Android panel, target/device pickers, toolbar controls and Run menu
actions. Schedule Build, Run, Test, Lint and Logcat in existing task terminals.
Reuse workspace trust and error handling; save edits before builds and deploy
only a successful selected-variant artifact to the selected serial. Remember the
selected module/variant in the local state database and restore it only against
a fresh model; a removed variant requires an explicit new selection.

Validation: native edit/build/run loop, intentional build failure and recovery,
terminal interruption, Logcat, and GPUI trust/root/device/dock/action coverage.
`cargo test -p android_ui` and `./script/clippy -p android_ui` pass.

Release Notes:

- Added Android build, run, test, lint, and device-log workflows

## 5. Arrange the workspace around Studio-style tool rails

Branch: `codex/android-ide/studio-shell`

Place existing dock controls on side tool rails, keep bottom output controls
together, and add Project-pane collapse/hide affordances. Use the existing panel
and docking mechanisms. Keep the center content at full height so the editor
cannot collapse when wrapped by the new rails.

Validation: native normal/small-window checks and the existing workspace dock
test with an added editor-geometry assertion. Shell Clippy passes with the
documented `gpui/inspector` feature flag.

Release Notes:

- Improved workspace layout for Android Studio users

## 6. Configure Kotlin with the selected Android classpath

Branch: `codex/android-ide/kotlin-setup`

Add explicit project-level Kotlin compatibility setup. Build the selected
variant, export evaluated Gradle libraries and generated Java output, write a
protected classpath hook, select the community Kotlin server with JDK 21, and
restart language support. Preserve existing hooks and unrelated settings.

Validation: generated `R`/`BuildConfig`, Compose symbols, Java helper use,
Android-library hover/navigation, configuration-cache reuse, no-Java variants,
and filesystem safety checks. Cross-module rename fails inside the deprecated
community server and is explicitly unsupported; this layer is a temporary
editing compatibility path, not full Android Studio language parity.

Release Notes:

- Added Kotlin compatibility setup for a selected Android build variant

## 7. Launch the fork with an isolated development profile

Branch: `codex/android-ide/dev-launcher`

Add `script/android-ide` to build/package/run a macOS development app with a
dedicated bundle ID and profile. Discover the existing Android SDK/JBR, preserve
existing profiles, and initialize new profiles with updates/telemetry disabled
and Kotlin support selected. Refuse to overwrite a running development bundle.

Validation: shell syntax, help/errors, debug/release launch, fresh and existing
profiles, paths relative to the caller, SDK discovery, and the running-app guard.

Release Notes:

- Added an isolated macOS launcher for Android IDE development

## 8. Start and stop a selected Android emulator

Branch: `codex/android-ide/emulator-start`

List existing SDK AVDs and start one through Android CLI in a task terminal.
Refresh devices on completion. Stop only the selected connected emulator,
preserving the AVD; disable stop for physical or unavailable devices.

Validation: native start, boot, run, stop, and empty-device refresh on
`medium_phone`; malformed AVD-name parsing and physical-device rejection tests.

Release Notes:

- Added existing-emulator startup and shutdown controls

## 9. Add a multi-module Android smoke project

Branch: `codex/android-ide/smoke-project`

Add a small reproducible Compose project with a non-`:app` application module,
demo/full flavors, an Android library dependency, generated symbols, a Java
helper, and a focused variant test. Pin and verify the Gradle wrapper. Keep
machine-specific settings, caches and build outputs ignored.

Validation: both debug builds and unit-test variants, lint with documented
warnings, native target selection and deployment, classpath export, library
hover and Go to Definition. The app visibly identifies its flavor and library.

Release Notes:

- Added a multi-module Android sample for testing IDE workflows

## 10. Record validation evidence and the local review stack

Branch: `codex/android-ide/validation`

Record exact tool versions, checks, reproduction commands, initial memory and
startup measurements, remaining product gaps, and local stack review steps.
Keep screenshots and raw performance/build logs in the ignored local validation
directory. Add the language-server findings to the implementation roadmap.

Validation: cross-check report claims against command logs, native screenshots,
source revisions, and branch ancestry. No publication occurs in this layer.

Release Notes:

- N/A
