import com.vanniktech.maven.publish.AndroidSingleVariantLibrary
import org.jetbrains.kotlin.gradle.dsl.JvmTarget

plugins {
    id("com.android.library")
    id("org.jetbrains.kotlin.android")
    id("com.vanniktech.maven.publish")
}

val noticeResources = tasks.register<Sync>("copyNotice") {
    from(rootProject.file("../../NOTICE")) {
        into("META-INF")
    }
    into(layout.buildDirectory.dir("generated/notice"))
}

tasks.named("preBuild") {
    dependsOn(noticeResources)
}

android {
    namespace = "me.sospedra.blasphem"
    compileSdk = 35
    sourceSets.getByName("main").resources.srcDir(layout.buildDirectory.dir("generated/notice"))
    packaging.resources.excludes.remove("/META-INF/NOTICE")

    defaultConfig {
        minSdk = 24
        consumerProguardFiles("consumer-rules.pro")
        testInstrumentationRunner = "androidx.test.runner.AndroidJUnitRunner"
    }

    compileOptions {
        sourceCompatibility = JavaVersion.VERSION_17
        targetCompatibility = JavaVersion.VERSION_17
    }
}

kotlin {
    compilerOptions {
        jvmTarget.set(JvmTarget.JVM_17)
    }
}

dependencies {
    api("org.jetbrains.kotlinx:kotlinx-coroutines-android:1.10.2")
    androidTestImplementation("androidx.test.ext:junit:1.3.0")
    androidTestImplementation("androidx.test:runner:1.7.0")
}

mavenPublishing {
    configure(AndroidSingleVariantLibrary(variant = "release", sourcesJar = true, publishJavadocJar = true))
}
