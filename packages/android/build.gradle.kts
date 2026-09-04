import com.android.build.gradle.LibraryExtension
import com.vanniktech.maven.publish.AndroidSingleVariantLibrary
import com.vanniktech.maven.publish.MavenPublishBaseExtension

buildscript {
    configurations.classpath {
        resolutionStrategy.failOnNonReproducibleResolution()
    }
}

plugins {
    id("com.android.library") version "8.13.1" apply false
    id("org.jetbrains.kotlin.android") version "2.2.21" apply false
    id("com.vanniktech.maven.publish") version "0.37.0" apply false
}

allprojects {
    configurations.configureEach {
        resolutionStrategy.failOnNonReproducibleResolution()
    }
    buildscript.configurations.configureEach {
        resolutionStrategy.failOnNonReproducibleResolution()
    }
}

val publishedGroup = providers.gradleProperty("GROUP").get()
val publishedVersion = providers.gradleProperty("VERSION_NAME").get()

// `:packs:<code>:pack` and `:packs:<code>:detect`: one asset each, no sources, no build file.
val dataModules = subprojects.filter { it.path.startsWith(":packs:") && it.path.split(":").size == 4 }

val descriptions = mapOf(
    "pack" to "the sparse table, the lexicon, and the rule-pack version",
    "detect" to "the slice of the language-identification model",
)

configure(dataModules) {
    val code = parent!!.name
    val kind = name
    val artifactId = "blasphem-$kind-$code"

    apply(plugin = "com.android.library")
    apply(plugin = "com.vanniktech.maven.publish")

    extensions.configure<LibraryExtension> {
        namespace = "me.sospedra.blasphem.$kind.$code"
        compileSdk = 35
        defaultConfig {
            minSdk = 24
        }
        // AGP drops META-INF/NOTICE from java resources by default; the data license needs it in the AAR.
        packaging {
            resources.excludes.remove("/META-INF/NOTICE")
        }
    }

    extensions.configure<MavenPublishBaseExtension> {
        configure(AndroidSingleVariantLibrary(variant = "release", sourcesJar = true, publishJavadocJar = true))
        coordinates(publishedGroup, artifactId, publishedVersion)
        pom {
            name.set(artifactId)
            description.set("blasphem data for $code: ${descriptions.getValue(kind)}. Ships as assets/blasphem/$code.$kind. Blasphem hashes word and character n-grams into sparse feature vectors. A linear classifier trained offline scores them with 16-bit weights. Lexicons and context rules contribute to the verdict. Detection runs locally without neural networks or cloud inference.")
            licenses {
                license {
                    name.set("CC-BY-NC-SA-4.0")
                    url.set("https://creativecommons.org/licenses/by-nc-sa/4.0/")
                    distribution.set("repo")
                }
            }
        }
    }
}
