# Android IDE prototype: validation and handoff

Session: 13–14 September 2026. Platform: Apple Silicon macOS. Changes are published
as draft stacked PRs following the author’s later authorization. Historical
checkpoints below retain their original measurements and limitations.

This is a working Android edit/build/run/debug/Compose-preview prototype with an
Android Studio-inspired shell. Android-aware Java editing and Kotlin object
rename also work on the fixture. It remains a development build: mixed-language
refactoring, advanced Kotlin debugging, broad project-model compatibility and
release packaging are not complete. The
[research and implementation plan](ANDROID_IDE_PLAN.md) covers the longer-term
work; [the review guide](ANDROID_IDE_REVIEW.md) describes the review stack.

## Run it

The optional language/debug/preview tools are installed in this checkout. To
reproduce their installation on Apple Silicon with JDK 21, run:

```sh
script/install-android-kotlin
script/install-android-debugger
script/install-android-preview
```

Each script verifies pinned source/artifact checksums, preserves an unmanaged
installation, and is repeatable. Kotlin and the debugger build with a task-local
JDK 11; execution uses JDK 21. Preview uses a separate short-lived JVM, without
starting Android Studio or an emulator.

From this checkout, launch the optimized app:

```sh
script/android-ide --release --skip-build examples/android-ide
```

The launcher creates a macOS development bundle under `target/android-ide` and
uses `target/android-ide/profile` for configuration and extensions. It preserves
existing settings. New profiles disable auto-update and telemetry, enable the
Kotlin and Java extensions, and select the community Kotlin server. This is a development
launcher, not a signed installer or an independently branded release.

1. Open the Android tool window using the hammer on the right tool rail.
2. Trusted Android projects sync automatically on open. Four runnable variants
   should appear for `:mobile`. After Gradle edits, save and choose **Sync project**.
   Sync reads files from disk;
   Build/Run/Test/Lint use the task system's save-before-run behavior.
3. Select **:mobile · demoDebug** or **:mobile · fullDebug**.
   The choice is remembered for this project and restored after the next sync.
4. Choose **Configure Kotlin**. It builds that variant, exports the evaluated
   compile classpath, and configures the project with a JDK 21 language server.
   The Kotlin extension must be installed; new launcher profiles request it
   automatically. Repeat setup after changing variants or dependencies.
5. Select a connected device or a stopped AVD in the top device picker.
   **Run** and **Debug** start a stopped selection and wait for it before deploying.
6. Choose **Run**. The app displays the selected flavor, its application ID,
   and `Android library connected`.
7. Use **Test**, **Lint**, or **Open Logcat** as needed. Build output remains in
   ordinary task terminals. Ctrl+C interrupts the focused command or Logcat.
8. Choose **Configure Java** for Android-aware Java editing; repeat Java and
   Kotlin setup after a variant/dependency change. Open a Java file to start its
   import, then allow JDT LS to finish before testing navigation.
9. Set Java/Kotlin breakpoints and choose **Debug**. Use the native debugger's
   variables, frames, step/continue and disconnect controls. The adapter supports
   local/field evaluation; arbitrary expression evaluation is not implemented.
10. Choose **Compose preview**, then **Select preview…** for Default or Large text.
    Refresh after editing. Preview works without a selected device and preserves
    the previous image on failure. Its editor split is reused across refreshes.
11. Use **Stop emulator** when finished. This preserves the AVD and releases its
   VM memory. It is disabled for a physical device.

The project is a real Gradle build and participates in Zed's existing workspace
trust flow. Sync and build execute project code only after the project is
trusted. Opening an arbitrary folder does not automatically build it or install
SDK packages.

## What is implemented and what was actually checked

| Area | Result and scope |
| --- | --- |
| Appearance | Studio Dark and Studio Light render in the native app. Bundled JetBrains Mono, compact typography, left Project pane, side tool rails, bottom output, and top Android controls are present. |
| Layout | Both themes checked at the normal window size; a smaller light-theme window retained an editor and scrollable Android controls. Dock geometry and panel movement have GPUI regression coverage. |
| Keybindings | Existing JetBrains map is the default. Native macOS Build, Run, Sync, and Logcat were exercised. Linux/Windows asset precedence is tested, but those operating systems were not run. |
| Project sync | Uses evaluated Android CLI output. Single-module reference and two-module smoke project both synchronize. Libraries are dependencies, not misleading runnable targets. |
| Variant handling | `:mobile` has demo/full flavors and debug/release build types. Both debug flavors build, test, and deploy with distinct application IDs. No `:app` assumption. |
| Variant restoration | The selected module/variant is stored in Zed's local database, separate from project files. A real quit/relaunch/Sync restored `:mobile · fullDebug`, and Run deployed that flavor without reselection. Fresh sync supplies current artifact paths. A removed variant remains unselected with a visible explanation. Tests cover project isolation and changed output paths. |
| Devices | Explicit serial selection; offline and unauthorized entries retained. Missing device state is visible. Physical-device stop is rejected in tests. |
| Emulator lifecycle | Native Start/Stop exercised on the existing `medium_phone` AVD. The optimized build also started `Pixel_6a` and deployed both flavors there. Stop refreshes the device list without deleting or recreating AVDs. |
| Build and Run | Save-before-build, selected variant, AGP output metadata, explicit device, and visible output. Build failure prevents deployment. Repeated run after fixing an intentional Kotlin type error showed the changed greeting on the emulator. |
| APK selection | Existing AGP metadata/redirects are read. The implementation accepts a single universal APK and rejects ambiguous, filtered/split, missing, traversing, or variant-mismatched artifacts. Split APK installation is deferred. |
| Test and Lint | Native task actions passed. Direct smoke-project builds/test tasks passed for both debug flavors; lint passed with warnings listed below. |
| Logcat | Real selected-device `adb logcat -v threadtime` in a terminal with bounded history. Ctrl+C stops the stream. No dedicated filter/table UI yet. |
| Kotlin Android APIs | Android/Compose resolution, useful hover/completion, and deliberate type-error diagnostics exercised on the reference project. |
| Generated symbols | Smoke `R`, `BuildConfig`, and a Kotlin call to the app's Java helper resolve after setup. Variant changes regenerate the selected `R.jar` and Java output classpath. |
| Library navigation | Native hover and Cmd+B navigate from `MainActivity.kt` to `LibraryGreeting.kt` in `:greeting`. The library's compiled classes JAR appears in the exported classpath. |
| Kotlin rename | The original 1.3.13 object-rename failure is fixed by the pinned upstream build. Native Shift+F6 changed the object, imports and usages across three Kotlin files; all temporary edits were restored. Java callers are not included, so mixed-language rename is not supported. |
| Java editing | Configure Java imports the evaluated AGP compile graph into JDT LS. Native SDK/resource/library hover, type errors and fullDebug-to-demoDebug BuildConfig navigation passed without clearing JDT caches. Existing preferences/comments and unrelated Gradle arguments are preserved. |
| Android Debug | Native build/deploy/debug-wait/attach passed. Java and Kotlin breakpoints, variables, Shift+F8 into the Compose caller and Ctrl+F2 detach were exercised. A portable DAP smoke test checks local evaluation, configuration completion, retained app process and removed owned forward. |
| Compose preview | Downloaded Google renderer/layoutlib produce real images in the native editor split. demoDebug/fullDebug use the correct resources, Java/Kotlin libraries and application IDs. Default/Large text selection works with no emulator and Android Studio closed. An intentional preview exception preserved the last image byte-for-byte and showed an error. |
| Cancellation | A temporary 60-second Gradle task was interrupted using Ctrl+C in the native terminal. Controls recovered, and the restored build passed in 2 seconds. Current task API displays interruption as command failure. |
| Configuration safety | Existing user hooks are preserved with an actionable error. Generated files use atomic replacement. Settings updates preserve unrelated settings/comments and reject stale concurrent settings edits. |
| Isolation | Development launcher profile, bundle identifiers, generated caches, telemetry/update defaults, existing-profile preservation, relative path handling, and running-app guard were checked. |
| Helper cleanup | After quitting the Java probe, a process check found no remaining IDE or language-server commands associated with the isolated profile. Shared Gradle/ADB services are separate; this is an app-quit check, not a project-close stress test. |

## Shortcut reference

| Operation | macOS | Linux/Windows mapping |
| --- | --- | --- |
| Find Action | Cmd+Shift+A | Ctrl+Shift+A |
| Build selected Android variant | Cmd+F9 | Ctrl+F9 |
| Run selected Android variant | Ctrl+R | Shift+F10 |
| Debug selected Android variant | Ctrl+D | Shift+F9 |
| Continue / step out / disconnect | F9 / Shift+F8 / Ctrl+F2 | Existing JetBrains debugger mappings |
| Sync Android project | Cmd+Option+Y | Ctrl+Alt+Y |
| Open selected-device Logcat | Cmd+6 | Alt+6 |

Sync originally lost to the default editor's Git Stage shortcut. The regression
test first reproduced `git::ToggleStaged` instead of `android::SyncProject`, then
passed after the Android bindings were given full-editor context as well as
workspace context. Native editor-focus Sync was rechecked after that fix.

## Language-server research changed the implementation choice

The official Kotlin LSP advertises experimental AGP support and remains Alpha;
KMP support is still under development. These are vendor-supported scope
statements, not evidence that this fixture works with that server.
[Kotlin documentation](https://kotlinlang.org/docs/kotlin-lsp.html).

The installed Zed Kotlin extension was version 0.3.2. Its default official
server download, `262.9593.0`, exited because that build had expired when tested
on 13 September. No expiry check was bypassed. JetBrains separately documents
30-day preview-build expiry for its Java/Kotlin VS Code integration and an
Ultimate subscription requirement after preview; do not assume this is a free,
unrestricted redistribution path for this fork.
[JetBrains installation and preview terms](https://www.jetbrains.com/help/intellij-vscode/get_started_vs_code.html).

The initial compatibility path used `fwcd/kotlin-language-server` 1.3.13 with
JDK 21 and its documented project classpath hook. JBR 25 produced a Java-version
parsing error, so Gradle and language-server Java runtimes are selected
separately. The community repository now describes itself as deprecated in
favor of the official server. Its successful diagnostics/navigation here do not
make it a sustainable production language engine.
[Community server status and classpath hooks](https://github.com/fwcd/kotlin-language-server).

The generated Gradle init task reads the selected Kotlin task's libraries and
the corresponding Java compiler output. The latter is necessary for generated
`BuildConfig` and Java helpers. It works when the Java task is `NO-SOURCE` and
with Gradle configuration-cache reuse. It does not parse Gradle source code or
change the user's Gradle build files.

Known ceiling: one selected variant supplies a workspace-wide classpath. This
does not model different variants in different modules, test-specific
classpaths, KMP, included builds, or source-set visibility with Android Studio's
precision. The pinned upstream build below resolves the reproduced rename crash;
a maintained production engine and broader project-model fixtures remain required.

### Fixes verified during the continuation

`script/install-android-kotlin` builds MIT-licensed upstream commit
`6d9e61b79d4631e75516def7ab7ff0d8b9310467`. Its existing front-end exception
fallback fixes the object-rename crash. The bootstrap runs upstream rename tests;
an additional disposable regression covered object rename with a shadowed
parameter. The native IDE then renamed the library object across Kotlin files.
This does not fix Java caller renaming or the workspace-wide classpath ceiling.
[Source revision](https://github.com/fwcd/kotlin-language-server/tree/6d9e61b79d4631e75516def7ab7ff0d8b9310467).

The default and experimental JDT LS Android imports initially failed on AGP 9.4.0.
Configure Java now exports each actual compile task in the selected Gradle task
graph, applies the standard Eclipse model, and updates the root project through
JDT LS. Updating only a module left stale Gradle arguments; refreshing the root
fixed full-to-demo navigation without deleting caches. The JRE container must be
preserved, and task dependencies cannot be read during `projectsEvaluated` under
Gradle 9. Those findings are covered by integration evidence and the review guide.
Java extension 6.8.26 / JDT LS 1.61.0 resolved SDK APIs, R, BuildConfig, compiled
Kotlin libraries and deliberate type errors in the native IDE.
[Java extension](https://github.com/zed-extensions/java),
[JDT LS](https://github.com/eclipse-jdtls/eclipse.jdt.ls).

Android debugging uses MIT-licensed `fwcd/kotlin-debug-adapter` revision
`7f05669b642d21afa46ac7b75307fa5d523a7263`, built with the pinned Kotlin server's
shared module and Kotlin 2.1.0. The checked-in patch completes configurationDone,
registers the intended class-prepare filters, removes obsolete breakpoint
requests, tolerates optional source names, and disposes attached VMs on detach.
The installed adapter passed upstream tests and a real emulator protocol test;
the native UI passed Java/Kotlin breakpoints and stepping. The ordinary Java DAP
adapter was separately proven, but its JDT source lookup did not resolve Kotlin
frames correctly, so it is not the default Android attach path.
[Adapter source](https://github.com/fwcd/kotlin-debug-adapter/tree/7f05669b642d21afa46ac7b75307fa5d523a7263).

A final native test replaced the adapter executable with `/usr/bin/false` for one
isolated process. That exposed a shared Zed DAP disconnect race. Closing request
registration on EOF/write failure, and isolating request maps across TCP
connections, fixes it. The deterministic regression failed before the change;
all four DAP tests pass afterward, including reconnect. The same native failure
now prints “debugger shutdown unexpectedly,” ends its session and removes the
forward automatically. Quitting the earlier failed instance also removed its
owned forward. The real adapter is restored simply by launching without that
process-local test override. Control-D was exercised with editor focus. The real
adapter was then rechecked: Java and Kotlin breakpoints, step-out and detach
passed; application PID 5187 remained alive with an empty forward list.

The cold-launch check also reproduced Android CLI returning before the new app
process exists. Process discovery now retries up to 20 times with a 250 ms gap
and a one-second timeout per ADB command, then surfaces the final failure. A
GPUI regression checks delayed success, bounded failure and invalid PID output.
Successful PID output is still validated before creating the forward. The final
debug build then reached both Java and Kotlin breakpoints on its first cold
launch after force-stopping the disposable full-flavor app.

Compose preview pins Google renderer `0.0.1-alpha15` and all layoutlib components
to `16.2.4`. The newer renderer alpha16 and published layoutlib 17.0.1 had an API
mismatch; mixing native layoutlib versions also failed. The tested matching set
renders independently of Android Studio. Google's compiled preview detector
supplies method names and parameters. A small bridge exits after the alpha15
renderer disposes its framework, because that version otherwise leaves worker
threads alive. Render failures are checked in the result JSON, not inferred from
an exit code. Runtime R classes come from the evaluated resource-processing task;
compile-only R classes omit dependency resource IDs required by Compose.
[Google screenshot tooling](https://developer.android.com/studio/preview/compose-screenshot-testing),
[release notes](https://developer.android.com/studio/preview/compose-screenshot-testing-release-notes).

## Reproducible build environment

| Component | Verified version/location |
| --- | --- |
| Host | Apple Silicon, 16 GiB RAM, macOS 26.6.2 |
| Rust | Repository-pinned 1.98.1, installed with rustfmt, Clippy, rust-src and rust-analyzer |
| Xcode | 26.6, build 17F113; Metal compiler available |
| Android Studio reference | Quail 4, 2026.1.4, `/Applications/Android Studio.app` |
| Gradle JVM used by launcher | Android Studio JBR 25.0.3 when `JAVA_HOME` is unset |
| Kotlin JVM | Temurin 21.0.12.1 in `/Library/Java/JavaVirtualMachines/temurin-21.jdk/Contents/Home` |
| Android CLI | 1.0.16261425, `/opt/homebrew/bin/android` |
| SDK | `/Users/anil/Library/Android/sdk`; platform 37 used for fixture |
| Smoke build | AGP 9.4.0, Gradle 9.6.1, Kotlin Compose plugin 2.2.10, Compose BOM 2026.02.01 |
| Tools installed for this session | Rust components, official gh-stack extension, task-local JDK 11, pinned Kotlin server/debugger builds, Google Compose renderer/layoutlib |
| Tools reused | Xcode/Metal, CMake, Ninja, Homebrew, Android Studio/SDK/CLI, JDK 21, ADB/emulator, telegram-send |

The upstream LiveKit build helper could not retrieve its WebRTC archive through
its normal download path. The matching official archive was downloaded directly
and its extracted headers/libraries were supplied through the helper's supported
`LK_CUSTOM_WEBRTC` override. A copy is retained at the following task-local path:

```sh
export LK_CUSTOM_WEBRTC="$PWD/target/android-ide/dependencies/webrtc-0001d84-4/mac-arm64-release"
cargo build --release -p zed --bin zed --locked -j 4
```

If that ignored cache is removed, retrieve the matching
[official WebRTC archive](https://github.com/zed-industries/livekit-rust-sdks/releases/download/webrtc-0001d84-4/webrtc-mac-arm64-release.zip)
again or let upstream's download work. Do not reuse these prebuilt libraries
after a LiveKit/WebRTC ABI update without checking the dependency's tag.
The launcher also honors `CARGO_TARGET_DIR` and `CARGO_BUILD_JOBS`.

The Gradle wrapper was regenerated using Gradle 9.6.1 and its JAR hash was
matched to the official checksum:

```text
wrapper JAR SHA-256:
497c8c2a7e5031f6aa847f88104aa80a93532ec32ee17bdb8d1d2f67a194a9c7
distribution SHA-256 (pinned in gradle-wrapper.properties):
9c0f7faeeb306cb14e4279a3e084ca6b596894089a0638e68a07c945a32c9e14
```

Sources: [wrapper checksum](https://downloads.gradle.org/distributions/gradle-9.6.1-wrapper.jar.sha256),
[distribution checksum](https://downloads.gradle.org/distributions/gradle-9.6.1-bin.zip.sha256).

## Checks performed

The following checks passed on the combined source changes, including the
shortcut-context and emulator-stop fixes:

```sh
cargo test -p android_tools -j 6
cargo test -p android_ui -j 6
cargo test -p workspace test_toggle_docks_and_panels -j 6
./script/clippy -p android_tools
./script/clippy -p android_ui
./script/clippy -p workspace -p project_panel -p ui --features gpui/inspector
bash -n script/android-ide
git diff --check
git -c core.whitespace=blank-at-eol,blank-at-eof,space-before-tab,cr-at-eol \
  diff --check main..HEAD -- . ':(exclude)assets/fonts/jetbrains-mono/OFL.txt'
```

The complete-stack whitespace check accepts the generated Windows wrapper's
CRLF endings and excludes the unmodified vendored font license, which has one
upstream trailing space. The pinned debugger patch preserves upstream context
whitespace through a scoped Git attribute. An unconfigured `git diff --check main..HEAD` reports
those vendor-file differences; the source/doc check above passes.

Set `LK_CUSTOM_WEBRTC` as above for commands that build the relevant native
dependencies. The core crate has two focused tests covering parsers/artifact
selection and the real filesystem/classpath hook. The Android GPUI test covers
trust, multiple roots, device state, action dispatch, dock movement, settings
preservation, persisted variant selection, and keymap precedence. The existing workspace dock regression was
extended to catch an editor that accidentally collapses after adding tool rails.

The shell Clippy command explicitly enables `gpui/inspector`: without it, the
upstream all-features UI build fails on its inspector derive feature wiring.
No unrelated upstream source fix was added. Release linking also emits an
upstream large-`__eh_frame` warning; Cargo reports future incompatibility in
`block` 0.1.6. These were warnings, not failed validation.

Smoke-project verification:

```sh
cd examples/android-ide
export JAVA_HOME='/Applications/Android Studio.app/Contents/jbr/Contents/Home'
export ANDROID_HOME="$HOME/Library/Android/sdk"
./gradlew :mobile:assembleDemoDebug :mobile:assembleFullDebug \
  :mobile:testDemoDebugUnitTest :mobile:testFullDebugUnitTest \
  :mobile:lintDemoDebug
```

With the Android library module added, this passed in 9 seconds: 135 actionable
tasks, 72 executed and 63 up to date. These are warm local timings, not a build
speed comparison. Lint had five warnings: newer Gradle, Compose BOM and Activity
versions exist; the smoke manifest has a backup-policy warning and no launcher
icon. No lint errors. Versions were kept consistent with the verified fixture
rather than expanding this test into dependency upgrades.

## Historical optimized builds and initial performance measurements

The initial ten-layer prototype compiled in **18 minutes 26 seconds** with four
jobs. That executable reported `1651d598ccc367f3b1a6dbbc46ef5612a9c59103` and
passed native restart/Sync variant restoration, emulator startup, and Ctrl+R
build/deploy of `:mobile · fullDebug` on `medium_phone`. Stop emulator completed
and ADB listed no devices. These results precede the Java/debugger/preview work.

An earlier optimized build passed in 18 minutes 53 seconds using four jobs and
reported `93d7661d5f36e208286d10dfd4be459332441142`. Both flavors deployed on
`Pixel_6a` after the library fixture was added. Repeated debug builds were used
for shorter test/fix cycles. Neither historical revision should be described as
the final continuation binary; the final build is recorded below.

### Warm startup observation

Three launches of the optimized `93d7661d5f` executable against the same already-used
profile and smoke project produced **0.367, 0.169, and 0.165 seconds** from process
creation to receiving Zed's first `Rendered first frame` log. Median: **0.169
seconds**. A monotonic Python timer read the child process's PTY; no UI-automation
tool latency is included. The actual sample-project window was checked after
each launch. No Rust compilation was running during these trials.

This log is emitted at the first workspace render call. It does not establish
frame presentation, editor input latency, project sync completion, or language
server readiness. These are warm-cache observations, with no Android Studio
startup comparison. Raw `startup-1.json` through `startup-3.json`, corresponding
logs, and the measurement script are retained in the local validation directory.

### Idle memory observation

Three samples were taken from 22:27:52 through 22:28:32 IST with the original
small Android Studio-generated Compose project open in both IDEs. Android Studio
and the optimized fork were idle; no Rust compiler was running during sampling.
The measurements used `vmmap` physical footprint because earlier compilation
had caused memory compression, making RSS alone misleading.

| Process or process group | Median physical footprint | Range |
| --- | ---: | ---: |
| Optimized Android IDE editor | 297.8 MiB | 297.7–297.8 MiB |
| Its Kotlin language server | approximately 1.30 GiB | rounded by vmmap |
| Editor plus Kotlin server | approximately 1.59 GiB | approximately 1.59 GiB |
| Android Studio | approximately 3.10 GiB | rounded by vmmap |
| Shared Gradle daemon (JBR 25) | approximately 1.80 GiB | rounded by vmmap |
| Shared Kotlin compiler daemon | 553.7 MiB | 553.7 MiB |
| Additional Gradle toolchain daemon | 701.6 MiB | 701.6 MiB |
| Shared emulator | approximately 5.30 GiB | rounded by vmmap |

The editor-plus-language-server group was about 49% smaller in these samples.
This is an initial observation on one small project, not a feature-equivalent
benchmark or a product-wide memory claim. Server functionality, JVM versions,
process age, compression, and warm caches differ. Gradle and emulator costs are
substantial and do not disappear when changing editors. Android Studio and the
fork may also cause distinct compatible/incompatible Gradle daemons to coexist.

The measured optimized executable reported revision `c0bf3f23`, an earlier
implementation build with the shell and Kotlin setup. Subsequent fixes add
generated Java outputs, emulator controls, and shortcut precedence. Do not label
this sample set as a measurement of the final source revision.

Raw JSON and all three per-process `vmmap` reports are retained in
`target/android-ide/validation/memory-baseline.json` and
`target/android-ide/validation/baseline-*.txt`. This ignored directory is local
evidence, not a committed binary payload.

## Continuation checks and evidence

The continuation adds four independent implementation layers after the initial
validation commit. Focused tests now cover four Android tool tests and three native
panel/debugger tests. Four DAP tests cover transport recovery, and five image-viewer
tests cover reload and asset lifetime behavior. The original
dock/panel tests remain relevant because the
new features reuse the existing workspace and debugger views.

```sh
cargo test -p dap -p android_tools -p android_ui -p image_viewer -j 6
./script/clippy -p dap -p android_tools -p android_ui -p project -p image_viewer \
  --features gpui/inspector -j 6
script/test-android-debugger --device emulator-5554
```

The optimized verification before the image-metadata correction used source
`a602e3b17768f5eebc8978373116493c5d47bb30`. That build passed in 7 minutes 47
seconds, restored fullDebug, rebuilt the app, configured both language servers,
and rendered Compose with Android Studio and the emulator closed. Preview-size
switching then exposed stale image metadata in the existing shared loader.

Final source `371380b5a7420ba8d937c9b642e8feaec3c1078e` refreshes metadata with the
new image bytes before notifying the existing viewer. Its real-file regression
failed with stale 1×1 dimensions before the change and passed with 2×1 afterward.
All five image-viewer tests passed, including asset-cache and split-pane checks.
The native rebuilt app then updated the existing pane from 681×399 to 845×480
and back, with the correct file size and working Fit to View.
The combined 16 tests and strict Clippy pass on that complete source. Its debug
build completed in 1 minute 10 seconds with the warm local cache. The final
documentation commit does not alter the native source. The optimized build of
this exact source completed successfully in **17 minutes 10 seconds** with six
jobs; only the documented upstream linker/future-compatibility warnings remain.

The debugger smoke requires the installed adapter, a running explicitly selected
emulator, and an assembled demoDebug fixture. It affects only the sample app and
its own port forward. Both demo/full local unit-test and lint tasks passed after
the named Compose previews were added. The intentional rendering exception was
removed and the original successful image remained unchanged throughout failure.

| Evidence under `target/android-ide/validation` | What it establishes |
| --- | --- |
| `java-summary.txt`, `java-native-sdk-hover.png`, `java-native-type-diagnostic.png` | Native Android Java resolution, diagnostics and variant import findings. |
| `kotlin-pinned-summary.txt`, `kotlin-upstream-native-rename.diff`, `kotlin-upstream-object-rename.png` | The actual Kotlin-only object rename and its scope. |
| `kotlin-pinned-install.log`, `kotlin-head-rename-tests.log` | Reproducible source install and upstream regression evidence. |
| `debugger-summary.txt`, `debugger-installed-smoke.log` | Java/Kotlin stops, local evaluation, handshake, retained app process and forward removal. |
| `dap-disconnect-before.log`, `dap-disconnect-after.log`, `debugger-startup-recovery.png` | Reproduced transport failure, fixed regression and native failure recovery. |
| `debug-native-kotlin-step.png` | Native stepping from the Kotlin library into its Compose caller. |
| `preview-summary.txt`, `compose-native-workspace.png` | Native standalone Compose rendering and side-by-side source/image layout. |
| `compose-native-full-default.png`, `compose-native-full-large.png` | Distinct default and large-text renders from fullDebug. |
| `preview-native-tests.log`, `preview-native-clippy.log` | Preview model, safe output handling and native panel checks. |
| `debug-process-wait-tests.log`, `debugger-cold-start-fixed.png` | Readiness regression and first-attempt native Java/Kotlin cold-start attachment. |
| `debugger-real-adapter-recovery.png` | Real-adapter stepping after the shared transport fix. |
| `final-sample-tests.log` | Both variants' unit tests and Android lint. |
| `final-combined-tests.log`, `final-combined-clippy.log` | All 16 focused tests and strict Clippy on the complete continuation source. |
| `final-debug-build.log`, `final-release-build.log`, `final-build.json` | Final-source debug and optimized compilation, with binary hash. |
| `final-optimized-workspace.png`, `final-memory.json`, `final-memory-summary.log` | Final running workspace and editor/language-server memory costs. |
| `preview-metadata-before.log`, `preview-metadata-after.log`, `preview-metadata-clippy.log` | Reproduced stale metadata and verified shared image reload fix. |
| `preview-metadata-native-large.png`, `preview-metadata-native-default.png` | Correct dimensions and file size after native preview-size switches. |

These artifacts are local and ignored. Telegram updates were sent to the
explicitly approved configured destination, including native debugger and Compose
images. They contain only this test IDE/project evidence.

## Final running build and process snapshot

The optimized `371380b5a7420ba8d937c9b642e8feaec3c1078e` executable is left open
on the smoke project. Sync restored `:mobile · fullDebug`, and the existing source
and real Compose preview are visible. Both language servers are running. Android
Studio and the test emulator are closed, and `adb forward --list` is empty.
The final workspace screenshot is `final-optimized-workspace.png`.

Binary SHA-256:
`05c66620bdb7b122d6318f3ea00f97b86f09756efd50e3f6efd8d83a5902b632`.
`final-build.json` records the source revision, size and completed checks.

Three physical-footprint samples were taken from 09:29:19 through 09:29:44 IST on
14 September with no Rust compiler running. These are post-startup observations
on the two-module fixture, not a controlled large-project benchmark. The first
sample still had editor/Kotlin activity; the latter two showed about 0.4% editor
CPU and 0% for the language servers.

| Process group | Median physical footprint | Range |
| --- | ---: | ---: |
| Optimized editor | 334.3 MiB | 322.3–386.8 MiB |
| Pinned Kotlin server | approximately 1.40 GiB | rounded by vmmap |
| Java language server | 695.6 MiB | 695.6–699.5 MiB |
| Java proxy | 1.48 MiB | 1.48 MiB |
| Editor plus Java/Kotlin support | approximately 2.41 GiB | approximately 2.40–2.46 GiB |

The owned terminal shell adds about 5.31 MiB. macOS denied `vmmap` access to its
protected `/usr/bin/login` parent; that footprint is recorded as unavailable,
not zero. Shared Gradle/ADB services are outside this process group. The earlier
Android Studio measurement used a different state and fixture, so no percentage
improvement over Studio is inferred from these final samples. Language-server
cost remains the main memory target for future work.

`final-memory.json`, `final-memory-summary.log`, and `final-memory-*.txt` retain
all measurements, CPU/RSS observations and the unavailable-helper result.
`measure-final-memory.py <editor-pid>` reproduces the scoped sample and verifies
that the PID belongs to the isolated optimized app.

## Remaining acceptance gates

1. **Language engine and project model:** a maintained production Kotlin engine,
   mixed Java/Kotlin refactoring, test/source-set visibility, KMP and included
   builds. The observed object rename and basic Android Java import failures are
   fixed; they are no longer pending implementation tasks.
2. **Advanced debugging:** coroutine/inline/SMAP mappings, complex expressions,
   deep multi-module source roots, process-death/reconnect stress and NDK/LLDB.
   Basic Java/Kotlin deploy/attach, variables, stepping and detach are implemented.
3. **Preview breadth:** XML design tools, interactive Compose/live edit,
   multi-value PreviewParameter galleries, larger dependency graphs and broader
   device/theme/locale matrices. Static Compose rendering, variant correctness,
   annotation selection and failure preservation are implemented.
4. **Project view and device tools:** logical Android grouping, native Logcat
   filters, AVD/SDK creation and management, split APK support, and an embedded
   emulator need their own tested increments.
5. **Performance:** repeat measurements on representative large projects, record
   input/frame latency and LSP-ready time, test repeated sync/close cycles, and
   include total helper-process cost. Deep file scanning is enabled so nested
   Kotlin files appear in projects without Git; its cost on large or overly
   broad roots still needs measurement. No responsiveness comparison is
   inferred from editor memory alone.
6. **Distribution:** validate Linux and Windows; separate product identity,
   logs/caches/update/crash policy from upstream; review license obligations;
   provide a signed installer and upgrade path. The current launcher is macOS
   development tooling only.

## Local evidence

The ignored `target/android-ide/validation` directory preserves the actual
screenshots and logs without adding build artifacts to the review stack:

| Artifact | What it demonstrates |
| --- | --- |
| `zed-android-studio-reference.png` | Installed Android Studio layout. This early capture predates recovery of the reference project's initial sync failure. |
| `zed-android-studio-shell.png` | Running native fork with Studio-style tool rails and build/run output. |
| `zed-final-native-run.png` | Initial ten-layer optimized executable after full-flavor deployment on `medium_phone`. |
| `saved-variant-restored.png` | `fullDebug` restored by Sync after a real app quit and relaunch. |
| `zed-android-studio-light.png` | Real light-theme editor, project tree, Android controls, and successful build. |
| `zed-android-narrow.png` | Smaller window with Android controls still reachable by scrolling. |
| `smoke-full-final.png` | The full flavor on Pixel_6a, including its application ID and Android library output. |
| `zed-android-cancel.png` | Deliberately interrupted Gradle task and the current interruption-as-failure UI. |
| `zed-android-rename-error.png`, `zed-kotlin-rename-server-error.png` | Reproduced community Kotlin rename failure. |
| `zed-java-android-import-error.png`, `jdtls-android-import.log`, `AndroidIdeJavaProbe.java` | Failed Android Java import after enabling JDT LS Android support. |
| `zed-final-combined-tests.log`, `zed-final-combined-clippy.log` | Combined stack checks after the variant-restoration change. |
| `zed-selection-release-build.log`, `zed-final-system-specs.txt` | Initial ten-layer optimized build and its source revision. |
| `memory-baseline.json`, `baseline-*.txt` | Three raw per-process memory samples. |
| `helper-cleanup.json` | Scoped process check after closing the development app and Java probe. |
| `startup-*.json`, `startup-*.log`, `zed-startup-measure.py` | Monotonic warm first-workspace-render measurements and harness. |

All requested progress messages were sent using `telegram-send` to the
explicitly approved configured destination. Screenshots show running local
software, not generated mockups. The root README review notice remains in place
for the human author to remove only after reviewing the stack.

## Review regression pass — 14 September 2026

### Fixes and focused evidence

- Dependency sources: Configure Kotlin exports the selected Gradle variant’s
  source archives. The pinned runtime reads the original Java/Kotlin file before
  trying decompilation. The disposable fixture resolved `ComponentActivity` to
  the original Java source, with a 1.22-second definition request in the recorded
  probe. This is one functional probe, not a performance benchmark.
- Resources: `R.string.app_name` and `@string/app_name` resolve from module XML
  buffers before calling the language server. The test covers locales, module
  boundaries, framework/foreign namespaces, ignored comments, and malformed XML.
  This works without waiting for a generated `R` class or an initialized server.
- Language installation: default automatic extensions now include Kotlin, Java
  and XML. The Java extension supplies Gradle Kotlin DSL highlighting; XML handles
  both the manifest and values files. User extension overrides remain respected.
- Repeated command-clicks: cached definition links move the caret to the actual
  clicked symbol before falling back to references. The existing picker now
  handles one reference and replaces an already-open query instead of closing it.
- Search Everywhere: actual Shift press/release pairs, class filtering, combined
  file/action/symbol results, category navigation and opening a class have GPUI
  coverage. Language-server failure is visible while file/action search remains
  available.
- Find/Replace: common Mac shortcuts and Ctrl+Shift+F/R aliases deploy a modal.
  Project-panel Find keeps the selected directory filter. The regression replaces
  three occurrences in two files, preserves tabs, cancels close, then saves safely.
- Automatic sync, selected stopped AVDs, trust boundaries and device eligibility
  are covered in the Android panel test. Debug shares the same emulator startup
  path as Run.

The combined run passed 128 tests: Android tools 4, Android UI 3, command palette
20, LSP locations 8, and search 93. The final resource navigation integration
test also passed. `cargo fmt --all -- --check` and the repository Clippy script
for all changed crates passed. Raw logs
and screenshots remain in ignored `target/android-ide/validation/review-*` files.

Re-run the focused checks from the repository root, with the installed WebRTC
path exported as in the build instructions above:

```sh
cargo test --locked -p search -p command_palette -p android_ui -p android_tools -p lsp_locations
cargo test --locked -p project --features test-support test_android_resource_definitions_without_language_server
cargo test --locked -p editor test_cached_declaration_click_moves_caret_before_usages
./script/clippy -p android_ui -p android_tools -p platform_title_bar -p project -p editor -p lsp_locations -p command_palette -p search -p project_panel --features gpui/inspector
```

### Shortcut audit

Checked the [Android Studio shortcut reference](https://developer.android.com/studio/intro/keyboard-shortcuts)
and [IntelliJ macOS keymap](https://www.jetbrains.com/help/idea/reference-keymap-mac-default.html).
The Mac map now corrects documentation, breakpoints, Resume, tab switching,
Version Control, block comments and auto-indent. Both maps retain separate Find
Usages results and the Show Usages popup. Search Everywhere and floating project
Find/Replace are implemented by the new layers, with existing editor features
used for navigation, folding, formatting, completion and rename.

This is not complete IntelliJ action parity: live templates, statement completion,
smart completion, Generate, and several structural refactorings need additional
language support. An untitled Zed buffer substitutes for a scratch file. macOS
may reserve Ctrl+Left/Right for Spaces; change the OS shortcut if it intercepts
editor-tab switching. Linux/Windows keymap loading is tested on macOS, not on
those operating systems.

### JetBrains Android plugin reference

The [resource model documentation](https://github.com/JetBrains/android/blob/master/android/src/com/android/tools/idea/res/README.md)
is useful for the next resource-navigation phase: it distinguishes module,
project, dependency and framework resources and explains source-set overlays.
The current implementation intentionally returns matching declarations in this
module’s source sets/locales, without pretending to choose the runtime qualifier
or selected-variant overlay. Resources in other modules/AARs fall back to LSP.

The [Android light-class documentation](https://github.com/JetBrains/android/blob/master/android/docs/android-light-classes.md)
explains why navigating to generated `R` code is the wrong editor destination
and how Studio redirects those symbols to their source resources before a build.
Our implementation follows that behavior through Zed’s existing definition
pipeline. The plugin’s PSI extension points are IntelliJ-specific; directly
embedding them into GPUI is not a small integration. No JetBrains plugin code was
copied. A Gradle-backed resource overlay model is the next step for namespace,
flavor, dependency and framework resource parity.
