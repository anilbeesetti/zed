# Android IDE smoke project

This small Compose app tests a module named `mobile`, two product flavors,
generated `R` and `BuildConfig` symbols, a Kotlin call into Java, and an Android
library dependency named `greeting`.

From the repository root, run:

```sh
script/android-ide --release examples/android-ide
```

Trusted Android projects sync automatically on open. Select **:mobile · demoDebug**
in the topbar and use **Configure Kotlin** in the Android tools panel. Install the Kotlin extension from Extensions if it is
not already installed. Select a connected device or a stopped emulator in the topbar,
then **Run**; a stopped emulator is booted before deployment. The app should display `dev.zed.androidsample.demo`. Repeat with
`fullDebug`; it should display `dev.zed.androidsample.full`. Both variants should
also display `Android library connected`. Run **Configure
Kotlin** again after switching variants so generated symbols use that variant.
Use **Stop emulator** when finished to release the VM's memory.

The Run menu's Android unit tests and lint actions operate on the selected
variant. To check the fixture directly:

```sh
cd examples/android-ide
./gradlew :mobile:assembleDemoDebug :mobile:testDemoDebugUnitTest :mobile:lintDemoDebug
```

The fixture requires Android SDK 37 and a JDK supported by Gradle 9.6.1. The
macOS launcher discovers the standard SDK and Android Studio runtime; command
line builds need `ANDROID_HOME` and `JAVA_HOME` set. Kotlin setup uses JDK 21
separately. Generated caches and machine-specific settings are ignored.

Run `script/install-android-kotlin`, `script/install-android-debugger`, and
`script/install-android-preview` once before launching the IDE. **Configure Java**
imports the selected variant into JDT LS; repeat Java and Kotlin setup after
changing variants or dependencies.

**Debug** builds and launches the selected app, then attaches the native debugger.
Set breakpoints on the return in `Greeting.java` and `LibraryGreeting.kt`; inspect
variables, step, and disconnect using the debugger controls. On macOS, Control-D
starts Android debugging, F9 continues, Shift-F8 steps out, and Control-F2
disconnects. `script/test-android-debugger --device emulator-5554` provides an
explicit emulator-only smoke test after building `demoDebug`.

**Compose preview** builds the selected variant and opens a rendered image beside
the code. **Select preview…** switches between the default and large-text
annotations. Rendering uses downloaded Google tooling, JDK 21, and the selected
variant's resources; Android Studio and a running device are unnecessary. Refresh
after code changes. Interactive previews and multi-value preview parameter
galleries are not implemented.
