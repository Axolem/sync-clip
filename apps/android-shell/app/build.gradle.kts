plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
}

fun envOrNull(name: String): String? =
    System.getenv(name)?.takeIf { it.isNotBlank() }

fun readVersionName(): String {
    // Gradle rootProject is apps/android-shell; VERSION lives at the monorepo root.
    val versionFile = rootProject.projectDir.resolve("../../VERSION").normalize()
    return if (versionFile.isFile) {
        versionFile.readText().trim().ifBlank { "0.1.0" }
    } else {
        "0.1.0"
    }
}

fun versionCodeFromName(versionName: String): Int {
    val parts = versionName.split(".", "-").mapNotNull { it.toIntOrNull() }
    val major = parts.getOrElse(0) { 0 }
    val minor = parts.getOrElse(1) { 0 }
    val patch = parts.getOrElse(2) { 0 }
    return major * 1_000_000 + minor * 1_000 + patch
}

val syncClipVersionName = readVersionName()
val syncClipVersionCode = versionCodeFromName(syncClipVersionName)

val releaseKeystorePath = envOrNull("SYNC_CLIP_ANDROID_KEYSTORE")
val releaseStorePassword = envOrNull("SYNC_CLIP_ANDROID_STORE_PASSWORD")
val releaseKeyAlias = envOrNull("SYNC_CLIP_ANDROID_KEY_ALIAS")
val releaseKeyPassword = envOrNull("SYNC_CLIP_ANDROID_KEY_PASSWORD")
val releaseKeystoreFile = releaseKeystorePath?.let { path -> file(path) }
val hasReleaseKeystore =
    releaseKeystoreFile != null &&
        releaseKeystoreFile.isFile &&
        !releaseStorePassword.isNullOrBlank() &&
        !releaseKeyAlias.isNullOrBlank() &&
        !releaseKeyPassword.isNullOrBlank()

android {
    namespace = "com.syncclip.shell"
    compileSdk = 36

    defaultConfig {
        applicationId = "com.syncclip.shell"
        minSdk = 26
        targetSdk = 36
        versionCode = syncClipVersionCode
        versionName = syncClipVersionName
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    signingConfigs {
        if (hasReleaseKeystore) {
            create("release") {
                storeFile = releaseKeystoreFile
                storePassword = releaseStorePassword
                keyAlias = releaseKeyAlias
                keyPassword = releaseKeyPassword
            }
        }
    }

    buildTypes {
        release {
            isMinifyEnabled = false
            if (hasReleaseKeystore) {
                signingConfig = signingConfigs.getByName("release")
            } else {
                // Sideload fallback for local builds — do not publish these APKs.
                logger.warn(
                    "SYNC_CLIP_ANDROID_KEYSTORE env incomplete; release APK will be signed with the debug keystore",
                )
                signingConfig = signingConfigs.getByName("debug")
            }
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    sourceSets {
        getByName("main") {
            jniLibs.srcDirs("src/main/jniLibs")
        }
    }
}

dependencies {
    implementation("androidx.appcompat:appcompat:1.7.0")
    implementation("androidx.core:core-ktx:1.15.0")
    implementation("androidx.security:security-crypto:1.1.0-alpha06")
    implementation("com.google.android.material:material:1.12.0")
    implementation("net.java.dev.jna:jna:5.15.0@aar")

    testImplementation("junit:junit:4.13.2")
    testImplementation("org.robolectric:robolectric:4.14.1")
}
