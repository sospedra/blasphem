import com.vanniktech.maven.publish.JavaPlatform

plugins {
    id("java-platform")
    id("com.vanniktech.maven.publish")
}

val publishedGroup = providers.gradleProperty("GROUP").get()
val publishedVersion = providers.gradleProperty("VERSION_NAME").get()
val dataModules = rootProject.subprojects.filter { it.path.startsWith(":packs:") && it.path.split(":").size == 4 }

dependencies {
    constraints {
        api("$publishedGroup:blasphem:$publishedVersion")
        for (module in dataModules) {
            api("$publishedGroup:blasphem-${module.name}-${module.parent!!.name}:$publishedVersion")
        }
    }
}

mavenPublishing {
    configure(JavaPlatform())
}
