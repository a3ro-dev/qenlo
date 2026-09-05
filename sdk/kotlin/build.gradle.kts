plugins {
    kotlin("jvm") version "2.2.20"
    kotlin("plugin.serialization") version "2.2.20"
    id("com.vanniktech.maven.publish") version "0.37.0"
}

group = "dev.qenlo"
version = "0.1.0-alpha.2"

repositories { mavenCentral() }

dependencies {
    api("net.java.dev.jna:jna:5.17.0")
    implementation("org.jetbrains.kotlinx:kotlinx-serialization-json:1.9.0")
    testImplementation(kotlin("test-junit5"))
    testRuntimeOnly("org.junit.jupiter:junit-jupiter-engine:5.13.4")
}

kotlin {
    explicitApi()
    jvmToolchain(17)
}

tasks.test {
    useJUnitPlatform()
    systemProperty("jna.library.path", System.getenv("QENLO_LIBRARY_DIR") ?: "")
}

mavenPublishing {
    publishToMavenCentral(automaticRelease = true)
    signAllPublications()
    coordinates(group.toString(), "qenlo", version.toString())
    pom {
        name.set("Qenlo Kotlin SDK")
        description.set("Typed Kotlin/JVM bindings for the embedded Qenlo vector database")
        inceptionYear.set("2026")
        url.set("https://github.com/a3ro-dev/qenlo")
        licenses {
            license {
                name.set("Apache License 2.0")
                url.set("https://www.apache.org/licenses/LICENSE-2.0.txt")
                distribution.set("https://www.apache.org/licenses/LICENSE-2.0.txt")
            }
            license {
                name.set("MIT License")
                url.set("https://opensource.org/licenses/MIT")
                distribution.set("https://opensource.org/licenses/MIT")
            }
        }
        developers {
            developer {
                id.set("a3ro-dev")
                name.set("Akshat Singh Kushwaha")
                email.set("akshatsingh14372@outlook.com")
                url.set("https://github.com/a3ro-dev")
            }
        }
        scm {
            url.set("https://github.com/a3ro-dev/qenlo")
            connection.set("scm:git:https://github.com/a3ro-dev/qenlo.git")
            developerConnection.set("scm:git:ssh://git@github.com/a3ro-dev/qenlo.git")
        }
    }
}
