# Android IDE smoke project

This small Compose app tests a module named `mobile`, two product flavors,
generated `R` and `BuildConfig` symbols, and a Kotlin call into Java.

From the repository root, run:

```sh
script/android-ide --release examples/android-ide
```

Use the Android tools panel to **Sync project**, select **:mobile · demoDebug**,
and **Configure Kotlin**. Install the Kotlin extension from Extensions if it is
not already installed. Select a connected device or use **Start emulator…**,
then **Run**. The app should display `dev.zed.androidsample.demo`. Repeat with
`fullDebug`; it should display `dev.zed.androidsample.full`. Run **Configure
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

The `@Preview` annotation is included for Android Studio reference testing.
This fork does not yet render Compose previews.
