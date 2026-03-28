plugins {
    id("com.android.application")
    id("org.jetbrains.kotlin.android")
    id("com.google.dagger.hilt.android")
    kotlin("kapt")
}

val apiBaseUrl = providers
    .gradleProperty("escudoApiBaseUrl")
    .orElse(providers.environmentVariable("ESCUDO_API_BASE_URL"))
    .orElse("https://api.escudovpn.com/")

val pinnedDomain = providers
    .gradleProperty("escudoPinnedDomain")
    .orElse(providers.environmentVariable("ESCUDO_PINNED_DOMAIN"))
    .orElse("api.escudovpn.com")

val certPins = providers
    .gradleProperty("escudoCertPins")
    .orElse(providers.environmentVariable("ESCUDO_CERT_PINS"))
    .orElse("sha256/5mMIaVKyfIokRH2im3ooEGrE6cRg68VGnKPRLpAvcqU=,sha256/y7xVm0TVJNahMr2sZydE2jQH8SquXV9yLF9seROHHHU=")

val releaseStoreFile = providers.environmentVariable("ESCUDO_UPLOAD_STORE_FILE")
val releaseStorePassword = providers.environmentVariable("ESCUDO_UPLOAD_STORE_PASSWORD")
val releaseKeyAlias = providers.environmentVariable("ESCUDO_UPLOAD_KEY_ALIAS")
val releaseKeyPassword = providers.environmentVariable("ESCUDO_UPLOAD_KEY_PASSWORD")
val hasReleaseSigning = releaseStoreFile.isPresent &&
    releaseStorePassword.isPresent &&
    releaseKeyAlias.isPresent &&
    releaseKeyPassword.isPresent

android {
    namespace = "com.escudo.vpn"
    compileSdk = 34

    defaultConfig {
        applicationId = "com.escudo.vpn"
        minSdk = 26
        targetSdk = 34
        versionCode = 1
        versionName = "1.0.0"

        buildConfigField("String", "API_BASE_URL", "\"${apiBaseUrl.get()}\"")
        buildConfigField("String", "PINNED_DOMAIN", "\"${pinnedDomain.get()}\"")
        buildConfigField("String", "CERT_PINS", "\"${certPins.get()}\"")
        multiDexEnabled = true
    }

    signingConfigs {
        create("release") {
            if (hasReleaseSigning) {
                storeFile = file(releaseStoreFile.get())
                storePassword = releaseStorePassword.get()
                keyAlias = releaseKeyAlias.get()
                keyPassword = releaseKeyPassword.get()
            }
        }
    }

    buildTypes {
        debug {
            buildConfigField("String", "API_BASE_URL", "\"${apiBaseUrl.get()}\"")
            buildConfigField("String", "PINNED_DOMAIN", "\"\"")
            buildConfigField("String", "CERT_PINS", "\"\"")
        }
        release {
            isMinifyEnabled = true
            isShrinkResources = true
            if (hasReleaseSigning) {
                signingConfig = signingConfigs.getByName("release")
            }
            proguardFiles(
                getDefaultProguardFile("proguard-android-optimize.txt"),
                "proguard-rules.pro"
            )
        }
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }

    kotlinOptions {
        jvmTarget = "17"
    }

    buildFeatures {
        compose = true
        buildConfig = true
    }

    composeOptions {
        kotlinCompilerExtensionVersion = "1.5.5"
    }
}

dependencies {
    // Compose BOM
    val composeBom = platform("androidx.compose:compose-bom:2023.10.01")
    implementation(composeBom)

    implementation("androidx.core:core-ktx:1.12.0")
    implementation("androidx.lifecycle:lifecycle-runtime-ktx:2.6.2")
    implementation("androidx.lifecycle:lifecycle-viewmodel-ktx:2.6.2")
    implementation("androidx.lifecycle:lifecycle-viewmodel-compose:2.6.2")
    implementation("androidx.activity:activity-compose:1.8.1")

    // Compose
    implementation("androidx.compose.ui:ui")
    implementation("androidx.compose.ui:ui-graphics")
    implementation("androidx.compose.ui:ui-tooling-preview")
    implementation("androidx.compose.material3:material3")
    implementation("androidx.compose.material:material-icons-extended")

    // Navigation
    implementation("androidx.navigation:navigation-compose:2.7.5")
    implementation("androidx.hilt:hilt-navigation-compose:1.1.0")

    // Hilt
    implementation("com.google.dagger:hilt-android:2.48.1")
    kapt("com.google.dagger:hilt-android-compiler:2.48.1")

    // Retrofit + OkHttp
    implementation("com.squareup.retrofit2:retrofit:2.9.0")
    implementation("com.squareup.retrofit2:converter-gson:2.9.0")
    implementation("com.squareup.okhttp3:okhttp:4.12.0")
    implementation("com.squareup.okhttp3:logging-interceptor:4.12.0")

    // Gson
    implementation("com.google.code.gson:gson:2.10.1")
    // Encrypted SharedPreferences
    implementation("androidx.security:security-crypto:1.1.0-alpha06")

    // WireGuard tunnel
    implementation("com.wireguard.android:tunnel:1.0.20230706")

    // Coroutines
    implementation("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.7.3")

    debugImplementation("androidx.compose.ui:ui-tooling")
    debugImplementation("androidx.compose.ui:ui-test-manifest")
}

kapt {
    correctErrorTypes = true
}

gradle.taskGraph.whenReady {
    val releaseTaskRequested = allTasks.any { task ->
        task.name.contains("Release", ignoreCase = true)
    }
    if (releaseTaskRequested && !hasReleaseSigning) {
        throw org.gradle.api.GradleException(
            "Release signing is required. Set ESCUDO_UPLOAD_STORE_FILE, ESCUDO_UPLOAD_STORE_PASSWORD, ESCUDO_UPLOAD_KEY_ALIAS, and ESCUDO_UPLOAD_KEY_PASSWORD."
        )
    }
}
