plugins {
    `java-gradle-plugin`
    `maven-publish`
    id("org.jetbrains.kotlin.jvm")
    id("com.vanniktech.maven.publish")
}

group = providers.gradleProperty("GROUP").get()
version = providers.gradleProperty("VERSION_NAME").get()
kotlin { jvmToolchain(17) }
mavenPublishing {
    configure(com.vanniktech.maven.publish.GradlePlugin())
    coordinates(group.toString(), "blasphem-gradle-plugin", version.toString())
}
dependencies { compileOnly("com.android.tools.build:gradle:8.13.1") }
tasks.processResources {
    from(rootProject.file("../../NOTICE")) { rename { "blasphem-NOTICE" } }
}
gradlePlugin {
    plugins {
        create("blasphem") {
            id = "me.sospedra.blasphem"
            implementationClass = "me.sospedra.blasphem.gradle.BlasphemPlugin"
            displayName = "Blasphem language distribution"
            description = "Select bundled or remote language data for the local Blasphem engine."
        }
    }
}
publishing {
    publications.withType<MavenPublication>().configureEach {
        pom {
            name.set("Blasphem Gradle plugin")
            description.set("Android language configuration and exact internal dependencies.")
            url.set("https://github.com/sospedra/blasphem")
            licenses { license { name.set("Apache-2.0"); url.set("https://www.apache.org/licenses/LICENSE-2.0.txt") } }
            developers { developer { id.set("sospedra"); name.set("Rubén Sospedra") } }
            scm { url.set("https://github.com/sospedra/blasphem") }
        }
    }
}
