pluginManagement {
    repositories {
        google()
        mavenCentral()
        gradlePluginPortal()
    }
}

dependencyResolutionManagement {
    repositories {
        google()
        mavenCentral()
    }
}

rootProject.name = "blasphem-android"

include(":engine", ":bom", ":gradle-plugin")

// One module per data file present. scripts/sync-packs.mjs exports resources/packs.
val codeDirectories = rootDir.resolve("packs").listFiles { file -> file.isDirectory }?.sorted() ?: emptyList()
for (codeDirectory in codeDirectories) {
    val code = codeDirectory.name
    for (kind in listOf("pack", "detect")) {
        if (codeDirectory.resolve("$kind/src/main/assets/blasphem/$code.$kind").isFile) {
            include(":packs:$code:$kind")
        }
    }
}
