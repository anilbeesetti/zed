# Local stack review guide

No branches have been pushed and no GitHub PRs have been created. These are
local `gh-stack` branches with review-ready descriptions. The current top is
`codex/android-ide/final-validation`; the base is local `main` at `7960b2a7c9568e90fbe0727332149e5b2a5fd57a`.

## Inspect and test the stack

```sh
gh stack view --json
git log --oneline main..codex/android-ide/final-validation
git diff --stat main..codex/android-ide/final-validation
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
| 10 | `validation` | `smoke-project` | `e9ccaabbea` |
| 11 | `java-support` | `validation` | `2faff91476` |
| 12 | `kotlin-runtime` | `java-support` | `66dc0548c9` |
| 13 | `debugger` | `kotlin-runtime` | `136b384657` |
| 14 | `compose-preview` | `debugger` | `371380b5a7` |
| 15 | `final-validation` | `compose-preview` | This report commit |

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
and filesystem safety checks. This layer initially reproduced a community-server
object-rename crash; layer 12 supplies the tested upstream fix. The compatibility
path still does not provide full Android Studio language parity.

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


## 11. Configure Android-aware Java editing from the selected Gradle variant

Branch: `codex/android-ide/java-support`

Default JDT LS import does not model the AGP 9 fixture correctly. Add Configure
Java to export the actual selected compile graph and import it through the
standard Eclipse/Buildship model. Preserve existing settings/comments, Gradle
arguments, bundles and the JRE container. Refresh the Gradle root when the server
becomes ready so changing variants updates cached Buildship arguments.

Validation: focused Rust/GPUI tests, Clippy and native build; SDK/resource/library
hover, type diagnostics and fullDebug-to-demoDebug BuildConfig navigation without
cache deletion. The initial failed import and successful replacement are both
recorded as evidence. Mixed-language refactoring remains outside this change.

### Suggested .rules additions

Proposed for Android/JDT integration rules: “When changing JDT LS Gradle import
arguments, send `java/projectConfigurationsUpdate` for the Gradle root URI.
Updating only module URIs can retain the previous root import arguments.”

Release Notes:

- Added selected-variant Android Java language support

## 12. Install a reproducible Kotlin runtime with the object rename fix

Branch: `codex/android-ide/kotlin-runtime`

The shipped community release crashes when renaming the fixture's Kotlin object.
Build a checksum-pinned upstream revision containing its existing front-end
fallback, run upstream rename tests, and install it under the task-local tool
cache. Keep build JDK 11 separate from runtime JDK 21 and enable the installed
server through the isolated launcher and Configure Kotlin.

Validation: clean source bootstrap, repeated installation, upstream rename tests,
native cross-file Kotlin rename, Rust tests, Clippy and native build. A disposable
regression checks the object and a shadowed parameter. Java callers are not part
of the Kotlin rename transaction; the community project's maintenance status
remains a production dependency risk.

Release Notes:

- Fixed the reproduced Kotlin object rename failure with a pinned runtime

## 13. Add native Android Java and Kotlin debugging

Branch: `codex/android-ide/debugger`

Debug now builds the selected variant, launches its validated APK through Android
CLI in debug mode, discovers its application process and creates an ephemeral
JDWP forward. Register the pinned community JVM adapter with Zed's existing
native debugger. Scope forward cleanup to that session and reject overlapping
Android debug launches. Add toolbar, tool-window, menu and JetBrains shortcuts.

The source patch fixes the adapter's configuration handshake, breakpoint class
filters, obsolete requests, optional source names and VM detach. The bootstrap
checks source hashes and runs upstream adapter tests. No new debug protocol or
expression engine is introduced.

Validation: native Java and Kotlin breakpoints, variables, step-out into the
Compose caller and disconnect. The app remains alive and the owned forward is
removed. `script/test-android-debugger --device emulator-5554` checks both source
languages, local evaluation, handshake and cleanup. Rust tests, Clippy and the
native build pass. Advanced Kotlin inline/coroutine mapping, arbitrary expressions
and NDK debugging are not claimed.

A controlled `/usr/bin/false` adapter exposed a shared DAP race: EOF only flushed
current callbacks, and write failures only logged, allowing a later initialize
request to wait forever. Close request registration on disconnect and give each
TCP connection its own request map so old I/O cannot close a replacement. The
regression fails before the change and passes afterward for stdio, TCP, pending
requests, later requests and reconnection. Native startup failure now reports an
error, ends the session and removes the owned Android forward automatically.

A cold launch also showed that Android CLI can finish before ActivityManager
creates the app process. Retry process discovery with bounded attempts and
per-command timeouts; preserve the final failure for the UI. A deterministic
test covers delayed success, exhaustion and invalid process IDs.

Release Notes:

- Added Android Java and Kotlin debugging in the native IDE

## 14. Render selected Android Compose previews beside source

Branch: `codex/android-ide/compose-preview`

Build the selected variant, export its evaluated runtime classes and resources,
and use Google's compiled preview detector and standalone renderer. The native
Android tool window offers Compose preview and annotation selection, opening the
result in a reusable editor split. A failed render preserves the previous image;
validated successful output replaces it atomically. Bound the renderer process
and dispose temporary files after each invocation.

Pin a compatible Google renderer/layoutlib set and compile a small Java bridge
that ensures the alpha15 process exits after disposing its framework. Preserve
existing project files and require workspace trust before running project code.
The fixture adds named Default and Large text previews.

Validation: Rust model/output/cache tests, native builds and Clippy; demoDebug and
fullDebug output, generated resources, Java/Kotlin library text, 845×480 large-text
and 681×399 default fullDebug images, selector changes, pane reuse and a deliberate
runtime failure preserving the image hash. Rendering works with Android Studio
closed and no emulator. Interactive/XML previews and multi-value parameter
galleries remain separate work.

Preview-size switching also exposed an existing image reload bug: pixels changed
while dimensions and file size stayed stale. Refresh metadata from the same new
bytes before emitting the existing reload event. A real-file GPUI regression
fails before the change; all five image-viewer tests pass afterward, including
asset-cache and split-pane checks. Project/image-viewer Clippy passes with the
existing inspector feature flag.

Release Notes:

- Added standalone Compose previews for the selected Android build variant

## 15. Record the final feature evidence and remaining product gates

Branch: `codex/android-ide/final-validation`

Update the original research checkpoint, local stack guide and validation report
with the completed Java/Kotlin/debugger/preview work, exact tool revisions,
reproduction steps and honest limits. Preserve earlier failures and performance
measurements as historical evidence rather than attributing them to the final
build. Keep all screenshots, logs and runtime downloads local and ignored.

Validation: cross-check the final build and feature evidence, local branch
ancestry, diff hygiene and absence of pushed branches or created PRs.

Release Notes:

- N/A
