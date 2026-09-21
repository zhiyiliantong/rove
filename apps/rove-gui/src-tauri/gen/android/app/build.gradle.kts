import java.util.Properties
import groovy.json.JsonSlurper

plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("rust")
}

val tauriProperties = Properties().apply {
    val propFile = file("tauri.properties")
    if (propFile.exists()) {
        propFile.inputStream().use { load(it) }
    }
}

android {
    compileSdk = 36
    namespace = "app.rove.desktop"
    defaultConfig {
        manifestPlaceholders["usesCleartextTraffic"] = "false"
        applicationId = "app.rove.desktop"
        minSdk = 24
        targetSdk = 36
        versionCode = tauriProperties.getProperty("tauri.android.versionCode", "1").toInt()
        versionName = tauriProperties.getProperty("tauri.android.versionName", "1.0")
    }
    buildTypes {
        getByName("debug") {
            manifestPlaceholders["usesCleartextTraffic"] = "true"
            isDebuggable = true
            isJniDebuggable = true
            isMinifyEnabled = false
            packaging {                jniLibs.keepDebugSymbols.add("*/arm64-v8a/*.so")
                jniLibs.keepDebugSymbols.add("*/armeabi-v7a/*.so")
                jniLibs.keepDebugSymbols.add("*/x86/*.so")
                jniLibs.keepDebugSymbols.add("*/x86_64/*.so")
            }
        }
        getByName("release") {
            isMinifyEnabled = true
            proguardFiles(
                *fileTree(".") { include("**/*.pro") }
                    .plus(getDefaultProguardFile("proguard-android-optimize.txt"))
                    .toList().toTypedArray()
            )
        }
    }
    kotlinOptions {
        jvmTarget = "1.8"
    }
    buildFeatures {
        buildConfig = true
    }
}

rust {
    rootDirRel = "../../../"
}

dependencies {
    // Resolve the Kotlin verifier from the exact Cargo dependency, not a
    // machine-specific cache path or an independently versioned download.
    val metadata = providers.exec {
        workingDir(projectDir)
        commandLine("cargo", "metadata", "--locked", "--format-version", "1",
            "--filter-platform", "aarch64-linux-android", "--manifest-path", "../../../Cargo.toml")
    }.standardOutput.asText.get()
    val packages = (JsonSlurper().parseText(metadata) as Map<*, *>)["packages"] as List<*>
    val verifier = packages.map { it as Map<*, *> }.single { it["name"] == "rustls-platform-verifier-android" }
    val verifierManifest = file(verifier["manifest_path"] as String)
    val verifierRepository = project.repositories.maven {
        url = uri(verifierManifest.parentFile.resolve("maven"))
        metadataSources { mavenPom(); artifact() }
    }
    project.repositories.exclusiveContent {
        forRepositories(verifierRepository)
        filter { includeGroup("rustls") }
    }
    implementation("rustls:rustls-platform-verifier:${verifier["version"]}")
    implementation("androidx.webkit:webkit:1.14.0")
    implementation("androidx.appcompat:appcompat:1.7.1")
    implementation("androidx.activity:activity-ktx:1.10.1")
    implementation("com.google.android.material:material:1.12.0")
    implementation("androidx.lifecycle:lifecycle-process:2.10.0")
    testImplementation("junit:junit:4.13.2")
    androidTestImplementation("androidx.test.ext:junit:1.1.4")
    androidTestImplementation("androidx.test.espresso:espresso-core:3.5.0")
}

apply(from = "tauri.build.gradle.kts")
